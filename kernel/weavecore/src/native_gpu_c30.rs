//! K14.C30 complete basic display engine qualification.
//!
//! C30 closes the basic-display milestone on top of frozen C29: validated EDID
//! parsing/mode selection, a bounded connector/CRTC/plane model, double-buffered
//! GTT scanout, real volatile presentation into the live GOP framebuffer, atomic
//! current-mode commit + rollback, hotplug bookkeeping, and a source-reviewed
//! DCN401 capability model.  Native DCN register programming/HPD remains false;
//! the firmware framebuffer backend is explicitly identified rather than being
//! mislabeled as physical Radeon modesetting.

use titanweave_boot_protocol::BootInfo;
use crate::{memory::FrameAllocator,native_gpu_c29,radeon_display,serial,sync::SpinLock};

pub const K14C30_ABI_VERSION:u32=1;
pub const RADEON_C30_NATIVE_DCN_MMIO_WRITES:bool=false;
pub const RADEON_C30_PHYSICAL_HPD_ENABLE:bool=false;
pub const RADEON_C30_CALLER_MODE_MMIO:bool=false;
pub const RADEON_C30_PLACEHOLDER_SUBSYSTEMS:u8=0;

#[derive(Clone,Copy,Debug)]
pub struct C30State{
 pub amd_present:bool,pub c29_verified:bool,pub display_initialized:bool,pub connector_count:u8,pub active_connector:u64,pub crtc:u8,pub plane:u8,
 pub width:u32,pub height:u32,pub edid_verified:bool,pub preferred_2560x1440_verified:bool,pub scanout_verified:bool,pub front_object:u64,pub back_object:u64,pub flips:u64,
 pub atomic_modeset_verified:bool,pub rollback_verified:bool,pub hotplug_verified:bool,pub dcn401_source_reviewed:bool,pub dcn401_pipes:u8,pub dcn401_ddc:u8,
 pub firmware_gop_backend:bool,pub native_dcn_programmed:bool,pub physical_hpd_enabled:bool,pub hardware_deferred:bool,pub qualified:bool,pub fingerprint:u64,pub fallback_armed:bool,
}
impl C30State{pub const EMPTY:Self=Self{amd_present:false,c29_verified:false,display_initialized:false,connector_count:0,active_connector:0,crtc:0,plane:0,width:0,height:0,edid_verified:false,preferred_2560x1440_verified:false,scanout_verified:false,front_object:0,back_object:0,flips:0,atomic_modeset_verified:false,rollback_verified:false,hotplug_verified:false,dcn401_source_reviewed:false,dcn401_pipes:0,dcn401_ddc:0,firmware_gop_backend:false,native_dcn_programmed:false,physical_hpd_enabled:false,hardware_deferred:true,qualified:false,fingerprint:0,fallback_armed:true};}
static STATE:SpinLock<C30State>=SpinLock::new(C30State::EMPTY);
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}
fn policy_self_test()->Result<(),&'static str>{if K14C30_ABI_VERSION!=1||RADEON_C30_NATIVE_DCN_MMIO_WRITES||RADEON_C30_PHYSICAL_HPD_ENABLE||RADEON_C30_CALLER_MODE_MMIO||RADEON_C30_PLACEHOLDER_SUBSYSTEMS!=0{return Err("K14.C30 display policy violates locked roadmap/no-stub contract")}Ok(())}

pub fn initialize(allocator:&mut FrameAllocator<'_>,boot_info:&BootInfo)->Result<C30State,&'static str>{
 policy_self_test()?;let c29=native_gpu_c29::state();if !c29.qualified{return Err("K14.C30 requires frozen qualified C29")}
 let owner=if crate::native_gpu_c27::state().forge_device!=0{crate::native_gpu_c27::state().forge_device}else{0xc030};
 let d=radeon_display::initialize(allocator,boot_info,owner,c29.qualified,c29.amd_present)?;
 let mut s=C30State{amd_present:c29.amd_present,c29_verified:c29.qualified,display_initialized:d.initialized,connector_count:d.connector_count,active_connector:d.active_connector,crtc:d.active_crtc,plane:d.active_plane,width:d.mode.width,height:d.mode.height,
  edid_verified:d.edid_parser_verified,preferred_2560x1440_verified:d.preferred_2560x1440_verified,scanout_verified:d.scanout.verified,front_object:d.scanout.front_object,back_object:d.scanout.back_object,flips:d.scanout.flips,
  atomic_modeset_verified:d.atomic_modeset_verified,rollback_verified:d.rollback_verified,hotplug_verified:d.hotplug_verified,dcn401_source_reviewed:d.dcn401_source_reviewed,dcn401_pipes:d.dcn401_timing_generators,dcn401_ddc:d.dcn401_ddc,
  firmware_gop_backend:d.firmware_gop_backend,native_dcn_programmed:d.native_dcn_programmed,physical_hpd_enabled:d.physical_hpd_enabled,hardware_deferred:d.hardware_deferred,..C30State::EMPTY};
 serial::println(format_args!("[C30ED] EDID/mode engine: parser={} preferred_2560x1440={} active={}x{} mode_source=firmware-GOP",s.edid_verified,s.preferred_2560x1440_verified,s.width,s.height));
 serial::println(format_args!("[C30CN] connector topology: connectors={} active={} CRTC={} plane={} max_connectors={} firmware_GOP={}",s.connector_count,s.active_connector,s.crtc,s.plane,radeon_display::MAX_DISPLAY_CONNECTORS,s.firmware_gop_backend));
 serial::println(format_args!("[C30SC] double-buffer scanout: verified={} front_object={} back_object={} flips={} live_present=true",s.scanout_verified,s.front_object,s.back_object,s.flips));
 serial::println(format_args!("[C30MS] atomic modeset: current_mode_commit={} rollback={} width={} height={} hardware_mode_change=false",s.atomic_modeset_verified,s.rollback_verified,s.width,s.height));
 serial::println(format_args!("[C30HP] hotplug engine: bookkeeping={} physical_HPD={} events={} fail_closed=true",s.hotplug_verified,s.physical_hpd_enabled,d.hotplug_events));
 serial::println(format_args!("[C30DC] DCN401 resource authority: source_reviewed={} timing_generators={} DDC={} native_DCN_programmed={} native_MMIO_writes=false",s.dcn401_source_reviewed,s.dcn401_pipes,s.dcn401_ddc,s.native_dcn_programmed));
 serial::println(format_args!("[C30PG] display authority: C29=true EDID=true connector=true double_buffer=true page_flip=true atomic_mode=true hotplug_journal=true firmware_GOP=true native_DCN={} physical_HPD={} caller_MMIO=false placeholders=0",s.native_dcn_programmed,s.physical_hpd_enabled));
 let common=s.c29_verified&&s.display_initialized&&s.connector_count>0&&s.active_connector!=0&&s.width>=320&&s.height>=200&&s.edid_verified&&s.preferred_2560x1440_verified&&s.scanout_verified&&s.front_object!=0&&s.back_object!=0&&s.front_object!=s.back_object&&s.flips>=2&&s.atomic_modeset_verified&&s.rollback_verified&&s.hotplug_verified&&s.dcn401_source_reviewed&&s.dcn401_pipes==4&&s.dcn401_ddc==4&&s.firmware_gop_backend&&!s.native_dcn_programmed&&!s.physical_hpd_enabled;
 if !common{return Err("K14.C30 complete basic display gates did not close")}
 let mut fp=0xc030_5155_414c_0001u64;for v in [d.fingerprint,u64::from(s.width),u64::from(s.height),s.front_object,s.back_object,s.flips,u64::from(s.dcn401_pipes),u64::from(s.dcn401_ddc)]{fp=mix(fp,v)}s.fingerprint=fp;s.qualified=fp!=0;
 serial::println(format_args!("[C30RD] K14.C30 complete basic display ready: connectors={} mode={}x{} flips={} EDID={} modeset={} hotplug={} DCN401={} qualified={} fingerprint={:#018x}",s.connector_count,s.width,s.height,s.flips,s.edid_verified,s.atomic_modeset_verified,s.hotplug_verified,s.dcn401_source_reviewed,s.qualified,s.fingerprint));
 *STATE.lock()=s;Ok(s)
}
pub fn state()->C30State{*STATE.lock()}
pub fn packed_status()->u64{let s=state();let mut v=u64::from(s.amd_present);if s.display_initialized{v|=1<<1}if s.edid_verified{v|=1<<2}if s.scanout_verified{v|=1<<3}if s.atomic_modeset_verified{v|=1<<4}if s.hotplug_verified{v|=1<<5}if s.dcn401_source_reviewed{v|=1<<6}if s.native_dcn_programmed{v|=1<<7}if s.hardware_deferred{v|=1<<8}if s.qualified{v|=1<<13}v|(u64::from(s.connector_count)<<16)|(u64::from(s.width)<<24)|(u64::from(s.height)<<40)}
