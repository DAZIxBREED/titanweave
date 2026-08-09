//! K14.C30 operational basic display engine.
//!
//! This module owns a bounded connector/CRTC/plane model, double-buffered GTT
//! scanout surfaces, actual volatile presentation into Titanweave's live GOP
//! framebuffer, atomic current-mode commits with rollback, and hotplug event
//! bookkeeping.  The firmware framebuffer is a real scanout backend, not a fake
//! native-DCN success.  Physical DCN401 programming remains explicitly false
//! until its exact register/transmitter prerequisites are reviewed and live.

use core::ptr;
use titanweave_boot_protocol::BootInfo;
use crate::{framebuffer::Framebuffer,memory::FrameAllocator,radeon_dcn401,radeon_memory,radeon_edid,sync::SpinLock};

pub const RADEON_DISPLAY_ABI_VERSION:u32=1;
pub const MAX_DISPLAY_CONNECTORS:usize=4;
pub const MAX_HOTPLUG_EVENTS:usize=16;
pub const MAX_SCANOUT_BYTES:u64=64*1024*1024;

#[repr(u8)]
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ConnectorKind{FirmwareGop=1,DisplayPort=2,Hdmi=3,Edp=4}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct DisplayMode{pub width:u32,pub height:u32,pub refresh_millihz:u32,pub pixel_clock_khz:u32}
impl DisplayMode{pub const EMPTY:Self=Self{width:0,height:0,refresh_millihz:0,pixel_clock_khz:0};pub const fn sane(self)->bool{self.width>=320&&self.height>=200&&self.width<=16384&&self.height<=16384&&self.refresh_millihz>=10_000&&self.refresh_millihz<=1_000_000}}
#[derive(Clone,Copy,Debug)]
pub struct Connector{pub id:u64,pub kind:ConnectorKind,pub connected:bool,pub crtc:u8,pub plane:u8,pub edid_valid:bool,pub mode:DisplayMode,pub active:bool}
impl Connector{pub const EMPTY:Self=Self{id:0,kind:ConnectorKind::FirmwareGop,connected:false,crtc:0,plane:0,edid_valid:false,mode:DisplayMode::EMPTY,active:false};}
#[derive(Clone,Copy,Debug)]
pub struct HotplugEvent{pub sequence:u64,pub connector:u64,pub connected:bool}
impl HotplugEvent{pub const EMPTY:Self=Self{sequence:0,connector:0,connected:false};}
pub struct HotplugJournal{events:[HotplugEvent;MAX_HOTPLUG_EVENTS],next:u64,count:usize}
impl HotplugJournal{pub const fn new()->Self{Self{events:[HotplugEvent::EMPTY;MAX_HOTPLUG_EVENTS],next:1,count:0}}pub fn record(&mut self,connector:u64,connected:bool)->Result<HotplugEvent,&'static str>{if connector==0{return Err("invalid hotplug connector")};let e=HotplugEvent{sequence:self.next,connector,connected};self.next=self.next.checked_add(1).ok_or("hotplug sequence exhausted")?;let idx=self.count%MAX_HOTPLUG_EVENTS;self.events[idx]=e;self.count=self.count.saturating_add(1);Ok(e)}pub fn count(&self)->usize{self.count}}

#[derive(Clone,Copy,Debug)]
pub struct ScanoutState{pub front_object:u64,pub back_object:u64,pub width:u32,pub height:u32,pub stride_bytes:u32,pub bytes:u64,pub flips:u64,pub present_hash:u64,pub live_hash:u64,pub verified:bool}
impl ScanoutState{pub const EMPTY:Self=Self{front_object:0,back_object:0,width:0,height:0,stride_bytes:0,bytes:0,flips:0,present_hash:0,live_hash:0,verified:false};}

#[derive(Clone,Copy,Debug)]
pub struct DisplayEngineState{
 pub initialized:bool,pub c29_verified:bool,pub connector_count:u8,pub active_connector:u64,pub active_crtc:u8,pub active_plane:u8,pub mode:DisplayMode,
 pub edid_parser_verified:bool,pub preferred_2560x1440_verified:bool,pub atomic_modeset_verified:bool,pub rollback_verified:bool,pub hotplug_verified:bool,pub hotplug_events:u64,
 pub scanout:ScanoutState,pub dcn401_source_reviewed:bool,pub dcn401_timing_generators:u8,pub dcn401_ddc:u8,pub native_dcn_programmed:bool,pub physical_hpd_enabled:bool,
 pub firmware_gop_backend:bool,pub hardware_deferred:bool,pub fingerprint:u64,
}
impl DisplayEngineState{pub const EMPTY:Self=Self{initialized:false,c29_verified:false,connector_count:0,active_connector:0,active_crtc:0,active_plane:0,mode:DisplayMode::EMPTY,edid_parser_verified:false,preferred_2560x1440_verified:false,atomic_modeset_verified:false,rollback_verified:false,hotplug_verified:false,hotplug_events:0,scanout:ScanoutState::EMPTY,dcn401_source_reviewed:false,dcn401_timing_generators:0,dcn401_ddc:0,native_dcn_programmed:false,physical_hpd_enabled:false,firmware_gop_backend:false,hardware_deferred:true,fingerprint:0};}
static STATE:SpinLock<DisplayEngineState>=SpinLock::new(DisplayEngineState::EMPTY);
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}

fn fill_pattern(base:u64,bytes:u64,width:u32,height:u32,stride:u32,variant:u32)->Result<u64,&'static str>{
 if base==0||u64::from(stride)*u64::from(height)>bytes{return Err("C30 scanout backing invalid")}
 let mut h=0xcbf29ce484222325u64;
 for y in 0..height { for x in 0..width {
  let pixel=if variant==0 { let r=(x.wrapping_mul(255)/width.max(1))&0xff;let b=(y.wrapping_mul(255)/height.max(1))&0xff;0x0008_1018u32 | (r<<16) | b }
             else { let g=(x.wrapping_add(y).wrapping_mul(255)/(width.saturating_add(height).max(1)))&0xff;0x0018_0808u32 | (g<<8) | ((x^y)&0xff) };
  let off=u64::from(y)*u64::from(stride)+u64::from(x)*4;if off+4>bytes{return Err("C30 scanout pattern overflow")}
  unsafe{ptr::write_volatile((base+off) as *mut u32,pixel)}
  if (x&127)==0&&(y&63)==0{h^=u64::from(pixel);h=h.wrapping_mul(0x100000001b3)}
 }} Ok(h)
}
fn atomic_commit(current:&mut DisplayMode,requested:DisplayMode,backend_mode:DisplayMode)->Result<(),&'static str>{
 if !requested.sane(){return Err("C30 atomic mode invalid")}
 if requested.width!=backend_mode.width||requested.height!=backend_mode.height{return Err("GOP backend cannot change firmware mode after ExitBootServices")}
 *current=requested;Ok(())
}
fn self_test_hotplug()->Result<usize,&'static str>{let mut j=HotplugJournal::new();let a=j.record(1,true)?;let b=j.record(2,true)?;let c=j.record(2,false)?;if a.sequence!=1||b.sequence!=2||c.sequence!=3||c.connected||j.count()!=3{return Err("C30 hotplug journal self-test failed")}Ok(j.count())}

pub fn initialize(allocator:&mut FrameAllocator<'_>,boot_info:&BootInfo,owner:u64,c29_qualified:bool,amd_present:bool)->Result<DisplayEngineState,&'static str>{
 if RADEON_DISPLAY_ABI_VERSION!=1||MAX_DISPLAY_CONNECTORS!=4||owner==0||!c29_qualified{return Err("C30 display prerequisites invalid")}
 let (edid,preferred)=radeon_edid::self_test()?;let caps=radeon_dcn401::capabilities()?;let hotplug_test=self_test_hotplug()?;
 let mut fb=Framebuffer::from_boot_info(boot_info)?;let info=fb.info();
 let mode=DisplayMode{width:info.width,height:info.height,refresh_millihz:60_000,pixel_clock_khz:0};if !mode.sane(){return Err("C30 GOP mode invalid")}
 let stride=info.width.checked_mul(4).ok_or("C30 scanout stride overflow")?;let bytes=u64::from(stride).checked_mul(u64::from(info.height)).ok_or("C30 scanout size overflow")?;
 if bytes==0||bytes>MAX_SCANOUT_BYTES{return Err("C30 scanout size outside bounded engine")}
 let a=radeon_memory::allocate_gtt(allocator,owner,bytes,4096)?;let b=match radeon_memory::allocate_gtt(allocator,owner,bytes,4096){Ok(v)=>v,Err(e)=>{radeon_memory::free(allocator,owner,a.id)?;return Err(e)}};
 radeon_memory::pin(owner,a.id,true)?;radeon_memory::pin(owner,b.id,true)?;
 let ah=fill_pattern(a.kernel_virtual,a.mapped_bytes,info.width,info.height,stride,0)?;let bh=fill_pattern(b.kernel_virtual,b.mapped_bytes,info.width,info.height,stride,1)?;if ah==bh{return Err("C30 scanout buffers are not distinct")}
 let p1=fb.copy_xrgb8888(a.kernel_virtual,stride,info.width,info.height)?;let live1=fb.sample_hash();let p2=fb.copy_xrgb8888(b.kernel_virtual,stride,info.width,info.height)?;let live2=fb.sample_hash();
 if p1==p2||live1==live2{return Err("C30 page-flip presentation did not change live scanout")}
 let mut active_mode=mode;atomic_commit(&mut active_mode,mode,mode)?;let before=active_mode;
 let bad=DisplayMode{width:mode.width.saturating_add(1),..mode};if atomic_commit(&mut active_mode,bad,mode).is_ok()||active_mode!=before{return Err("C30 atomic rollback verification failed")}
 let mut journal=HotplugJournal::new();let e=journal.record(1,true)?;if e.sequence!=1||journal.count()!=1{return Err("C30 live connector hotplug bookkeeping failed")}
 let connector=Connector{id:1,kind:ConnectorKind::FirmwareGop,connected:true,crtc:0,plane:0,edid_valid:false,mode,active:true};
 if !connector.active||!connector.connected{return Err("C30 active connector construction failed")}
 let scan=ScanoutState{front_object:b.id,back_object:a.id,width:info.width,height:info.height,stride_bytes:stride,bytes,flips:2,present_hash:p2,live_hash:live2,verified:true};
 let mut fp=0xc030_4449_5350_0001u64;for v in [u64::from(mode.width),u64::from(mode.height),a.id,b.id,p1,p2,live1,live2,edid.fingerprint,caps.fingerprint,u64::from(hotplug_test as u32)]{fp=mix(fp,v)}
 let s=DisplayEngineState{initialized:true,c29_verified:true,connector_count:1,active_connector:1,active_crtc:0,active_plane:0,mode,edid_parser_verified:edid.valid&&edid.mode_count>0,
  preferred_2560x1440_verified:preferred.width==2560&&preferred.height==1440,atomic_modeset_verified:true,rollback_verified:true,hotplug_verified:true,hotplug_events:1,scanout:scan,
  dcn401_source_reviewed:caps.source_reviewed,dcn401_timing_generators:caps.timing_generators,dcn401_ddc:caps.ddc,native_dcn_programmed:false,physical_hpd_enabled:false,
  firmware_gop_backend:true,hardware_deferred:amd_present,fingerprint:fp};
 *STATE.lock()=s;Ok(s)
}
pub fn state()->DisplayEngineState{*STATE.lock()}
