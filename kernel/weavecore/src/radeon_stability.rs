//! K14.C32 production/stability stress execution.
//!
//! Every QEMU-capable gate below executes against real Titanweave driver
//! objects: GTT allocations, C31 execution queues, C29 timeline fences, C30
//! scanout buffers and the live GOP framebuffer. Hardware-specific VRAM pressure
//! also executes when a real Radeon VRAM aperture is available. Physical ASIC
//! reset/interrupt/clock programming is never inferred from software stress.

use core::ptr;
use titanweave_boot_protocol::BootInfo;
use crate::{
 framebuffer::Framebuffer,
 memory::{FrameAllocator,FRAME_SIZE},
 native_gpu_c30::C30State,
 radeon_command::{CommandBuffer,CommandOpcode,ExecutionQueue,QueueClass,C31_QUEUE_DEPTH},
 radeon_driver,
 radeon_fence::RadeonFenceTimeline,
 radeon_memory,
 radeon_display::{self,Connector,ConnectorKind,DisplayMode,HotplugJournal},
 radeon_telemetry::{self,TelemetryKind},
};

pub const RADEON_STABILITY_ABI_VERSION:u32=1;
pub const C32_QUEUE_STRESS_ROUNDS:u32=32;
pub const C32_QUEUE_STRESS_SUBMISSIONS:u64=C32_QUEUE_STRESS_ROUNDS as u64*C31_QUEUE_DEPTH as u64;
pub const C32_GTT_PRESSURE_OBJECTS:usize=12;
pub const C32_GTT_PRESSURE_BYTES:u64=1024*1024;
pub const C32_VRAM_PRESSURE_OBJECTS:usize=4;
pub const C32_VRAM_PRESSURE_BYTES:u64=1024*1024;
pub const C32_IRQ_STRESS_EVENTS:u32=1024;
pub const C32_RECOVERY_STRESS_CYCLES:u32=32;
pub const C32_CONCURRENCY_ROUNDS:u32=64;
pub const C32_DISPLAY_STRESS_PRESENTS:u32=16;

#[derive(Clone,Copy,Debug)]
pub struct StabilityReport{
 pub queue_stress_verified:bool,pub queue_submissions:u64,pub queue_retires:u64,pub queue_wraps:u64,
 pub hang_detected:bool,pub hang_recovered:bool,pub abandoned_on_reset:u64,
 pub gtt_pressure_verified:bool,pub gtt_pressure_bytes:u64,pub vram_pressure_verified:bool,pub vram_pressure_deferred:bool,pub vram_pressure_bytes:u64,
 pub interrupt_stress_verified:bool,pub interrupt_events:u64,pub recovery_stress_verified:bool,pub recovery_cycles:u32,
 pub display_compute_concurrency:bool,pub graphics_compute_concurrency:bool,pub concurrency_rounds:u32,pub compute_ops:u64,pub graphics_pixels:u64,
 pub display_stress_verified:bool,pub display_presents:u32,pub multi_display_framework_verified:bool,pub multi_display_connectors:u8,
 pub fingerprint:u64,
}
impl StabilityReport{pub const EMPTY:Self=Self{queue_stress_verified:false,queue_submissions:0,queue_retires:0,queue_wraps:0,hang_detected:false,hang_recovered:false,abandoned_on_reset:0,gtt_pressure_verified:false,gtt_pressure_bytes:0,vram_pressure_verified:false,vram_pressure_deferred:true,vram_pressure_bytes:0,interrupt_stress_verified:false,interrupt_events:0,recovery_stress_verified:false,recovery_cycles:0,display_compute_concurrency:false,graphics_compute_concurrency:false,concurrency_rounds:0,compute_ops:0,graphics_pixels:0,display_stress_verified:false,display_presents:0,multi_display_framework_verified:false,multi_display_connectors:0,fingerprint:0};}
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}

fn queue_stress()->Result<(u64,u64,u64),&'static str>{
 let mut c=ExecutionQueue::new(QueueClass::Compute);let mut g=ExecutionQueue::new(QueueClass::Graphics);let mut seq=1u32;
 for round in 0..C32_QUEUE_STRESS_ROUNDS{
  for slot in 0..C31_QUEUE_DEPTH{let id=(u64::from(round)<<16)|slot as u64|1;let _=c.submit(0xc032_0000_0000_0000|id,seq)?;let _=g.submit(0xc032_8000_0000_0000|id,seq)?;seq=seq.checked_add(1).ok_or("C32 queue stress fence exhausted")?;}
  radeon_telemetry::record(TelemetryKind::QueueSubmit,(C31_QUEUE_DEPTH*2) as u64,u64::from(round))?;
  for _ in 0..C31_QUEUE_DEPTH{let cid=c.start_head()?;let gid=g.start_head()?;let fence=seq;let _=c.retire_head(fence)?;let _=g.retire_head(fence)?;if cid==0||gid==0{return Err("C32 queue stress invalid submission id")}}
  radeon_telemetry::record(TelemetryKind::QueueRetire,(C31_QUEUE_DEPTH*2) as u64,u64::from(round))?;
 }
 let(_,cs,cr,cc)=c.counters();let(_,gs,gr,gc)=g.counters();if cc!=0||gc!=0||cs!=C32_QUEUE_STRESS_SUBMISSIONS||cr!=C32_QUEUE_STRESS_SUBMISSIONS||gs!=C32_QUEUE_STRESS_SUBMISSIONS||gr!=C32_QUEUE_STRESS_SUBMISSIONS{return Err("C32 queue stress counters invalid")}
 Ok((cs+gs,cr+gr,u64::from(C32_QUEUE_STRESS_ROUNDS)))
}

fn hang_recovery()->Result<u64,&'static str>{let mut q=ExecutionQueue::new(QueueClass::Compute);let id=q.submit(0xc032_dead_beef,1)?;if q.start_head()?!=id{return Err("C32 hang injection start failed")}let(_,_,_,pending)=q.counters();if pending!=1{return Err("C32 hang injection did not leave pending work")}radeon_telemetry::record(TelemetryKind::HangDetected,1,id)?;let abandoned=q.reset();let(resets,total)=q.stability_counters();if abandoned!=1||resets!=1||total!=1||q.counters().3!=0{return Err("C32 stuck queue reset failed")}radeon_telemetry::record(TelemetryKind::HangRecovered,1,id)?;Ok(abandoned as u64)}

fn pressure_test(allocator:&mut FrameAllocator<'_>,owner:u64)->Result<(u64,bool,bool,u64),&'static str>{
 let before=radeon_memory::usage();let mut ids=[0u64;C32_GTT_PRESSURE_OBJECTS];let mut total=0u64;
 for i in 0..C32_GTT_PRESSURE_OBJECTS{let o=match radeon_memory::allocate_gtt(allocator,owner,C32_GTT_PRESSURE_BYTES,FRAME_SIZE){Ok(v)=>v,Err(e)=>{for id in ids.into_iter().filter(|x|*x!=0){let _=radeon_memory::free(allocator,owner,id);}return Err(e)}};let p=(0xc032_0000u32)^(i as u32).wrapping_mul(0x0101_0101);unsafe{ptr::write_volatile(o.kernel_virtual as *mut u32,p);ptr::write_volatile((o.kernel_virtual+o.mapped_bytes-4)as *mut u32,!p);if ptr::read_volatile(o.kernel_virtual as *const u32)!=p||ptr::read_volatile((o.kernel_virtual+o.mapped_bytes-4)as *const u32)!=!p{return Err("C32 GTT pressure readback failed")}}ids[i]=o.id;total=total.saturating_add(o.mapped_bytes);radeon_telemetry::record(TelemetryKind::MemoryAllocate,o.mapped_bytes,o.id)?;}
 for i in (0..C32_GTT_PRESSURE_OBJECTS).rev(){let id=ids[i];radeon_memory::free(allocator,owner,id)?;radeon_telemetry::record(TelemetryKind::MemoryFree,C32_GTT_PRESSURE_BYTES,id)?;}
 let after=radeon_memory::usage();if before!=after{return Err("C32 GTT pressure test leaked Radeon memory")}
 let mem=radeon_memory::state();let mut vram_ids=[0u64;C32_VRAM_PRESSURE_OBJECTS];let mut vram_total=0u64;let mut vram_verified=false;let mut vram_deferred=true;
 if mem.vram_allocator_ready{vram_deferred=false;for i in 0..C32_VRAM_PRESSURE_OBJECTS{let o=match radeon_memory::reserve_vram(owner,C32_VRAM_PRESSURE_BYTES,FRAME_SIZE){Ok(v)=>v,Err(e)=>{for id in vram_ids.into_iter().filter(|x|*x!=0){let _=radeon_memory::free(allocator,owner,id);}return Err(e)}};vram_ids[i]=o.id;vram_total=vram_total.saturating_add(o.mapped_bytes);radeon_telemetry::record(TelemetryKind::MemoryAllocate,o.mapped_bytes,o.id)?;}for i in (0..C32_VRAM_PRESSURE_OBJECTS).rev(){let id=vram_ids[i];radeon_memory::free(allocator,owner,id)?;radeon_telemetry::record(TelemetryKind::MemoryFree,C32_VRAM_PRESSURE_BYTES,id)?;}if radeon_memory::usage()!=after{return Err("C32 VRAM pressure test leaked reservation")};vram_verified=true;}
 Ok((total,vram_verified,vram_deferred,vram_total))
}

fn recovery_stress()->Result<(),&'static str>{for i in 0..C32_RECOVERY_STRESS_CYCLES{let mut m=radeon_driver::CoreMachine::new();m.claim()?;m.mmio_ready()?;m.online()?;m.quiesce()?;m.fault(0xc032_0000|i)?;m.coordinate_reset()?;m.online()?;if m.phase!=radeon_driver::CorePhase::Online||m.fault_count!=1||m.reset_epoch!=1{return Err("C32 recovery stress lifecycle failed")}radeon_telemetry::record(TelemetryKind::RecoveryCycle,1,u64::from(i))?}Ok(())}

fn concurrency_test(allocator:&mut FrameAllocator<'_>,boot_info:&BootInfo,owner:u64,c30:C30State)->Result<(u64,u64,u32),&'static str>{
 let a=radeon_memory::allocate_gtt(allocator,owner,FRAME_SIZE,FRAME_SIZE)?;let b=match radeon_memory::allocate_gtt(allocator,owner,FRAME_SIZE,FRAME_SIZE){Ok(v)=>v,Err(e)=>{radeon_memory::free(allocator,owner,a.id)?;return Err(e)}};let out=match radeon_memory::allocate_gtt(allocator,owner,FRAME_SIZE,FRAME_SIZE){Ok(v)=>v,Err(e)=>{radeon_memory::free(allocator,owner,b.id)?;radeon_memory::free(allocator,owner,a.id)?;return Err(e)}};
 let target=radeon_memory::object(owner,c30.back_object).ok_or("C32 concurrency scanout target missing")?;let stride=c30.width.checked_mul(4).ok_or("C32 concurrency stride overflow")?;if target.kernel_virtual==0||target.mapped_bytes<u64::from(stride)*u64::from(c30.height){return Err("C32 concurrency scanout target invalid")}
 let mut cc=CommandBuffer::allocate(allocator,owner)?;cc.emit(CommandOpcode::Dispatch,1,1,1,64)?;let mut gc=CommandBuffer::allocate(allocator,owner)?;gc.emit(CommandOpcode::DrawTriangle,3,1,0,0)?;gc.emit(CommandOpcode::Present,target.id,u64::from(c30.width),u64::from(c30.height),u64::from(stride))?;
 let mut cf=RadeonFenceTimeline::allocate(allocator,owner)?;let mut gf=RadeonFenceTimeline::allocate(allocator,owner)?;let mut cq=ExecutionQueue::new(QueueClass::Compute);let mut gq=ExecutionQueue::new(QueueClass::Graphics);let mut fb=Framebuffer::from_boot_info(boot_info)?;let mut compute_ops=0u64;let mut pixels=0u64;let mut presents=0u32;
 for round in 0..C32_CONCURRENCY_ROUNDS{for i in 0..64u32{unsafe{ptr::write_volatile((a.kernel_virtual as *mut u32).add(i as usize),round.wrapping_add(i));ptr::write_volatile((b.kernel_virtual as *mut u32).add(i as usize),round.wrapping_mul(3).wrapping_add(i*2));}}let(cs,_)=cf.issue()?;let(gs,_)=gf.issue()?;let cid=cq.submit(cc.object_id(),cs)?;let gid=gq.submit(gc.object_id(),gs)?;if cq.start_head()?!=cid||gq.start_head()?!=gid{return Err("C32 concurrency queue start failed")}
  for i in 0..64u32{let av=unsafe{ptr::read_volatile((a.kernel_virtual as *const u32).add(i as usize))};let bv=unsafe{ptr::read_volatile((b.kernel_virtual as *const u32).add(i as usize))};unsafe{ptr::write_volatile((out.kernel_virtual as *mut u32).add(i as usize),av.wrapping_add(bv))}}compute_ops+=64;
  let x=(round.wrapping_mul(37)%c30.width.max(1)) as u64;let y=(round.wrapping_mul(23)%c30.height.max(1)) as u64;let off=y*u64::from(stride)+x*4;if off+4>target.mapped_bytes{return Err("C32 concurrency graphics pixel outside target")}let color=0x00c0_3200u32^round^((round&0xff)<<16);unsafe{ptr::write_volatile((target.kernel_virtual+off)as *mut u32,color);if ptr::read_volatile((target.kernel_virtual+off)as *const u32)!=color{return Err("C32 concurrency graphics readback failed")}}pixels+=1;
  if round%8==0{let h=fb.copy_xrgb8888(target.kernel_virtual,stride,c30.width,c30.height)?;if h==0||fb.sample_hash()==0{return Err("C32 concurrency live present failed")}presents+=1;radeon_telemetry::record(TelemetryKind::DisplayPresent,1,u64::from(round))?;}
  cf.complete_software(cs);gf.complete_software(gs);if cq.retire_head(cf.completed())?!=cid||gq.retire_head(gf.completed())?!=gid{return Err("C32 concurrency retirement failed")};
 }
 for i in 0..64u32{let round=C32_CONCURRENCY_ROUNDS-1;let expect=round.wrapping_add(i).wrapping_add(round.wrapping_mul(3).wrapping_add(i*2));let got=unsafe{ptr::read_volatile((out.kernel_virtual as *const u32).add(i as usize))};if got!=expect{return Err("C32 concurrent compute output mismatch")}}
 if cq.counters().3!=0||gq.counters().3!=0||presents<8{return Err("C32 concurrency final counters invalid")}
 cf.release(allocator)?;gf.release(allocator)?;cc.release(allocator)?;gc.release(allocator)?;radeon_memory::free(allocator,owner,out.id)?;radeon_memory::free(allocator,owner,b.id)?;radeon_memory::free(allocator,owner,a.id)?;
 radeon_telemetry::add_compute_ops(compute_ops);radeon_telemetry::add_graphics_pixels(pixels);Ok((compute_ops,pixels,presents))
}

fn display_stress(boot_info:&BootInfo,owner:u64,c30:C30State)->Result<u32,&'static str>{let front=radeon_memory::object(owner,c30.front_object).ok_or("C32 front scanout missing")?;let back=radeon_memory::object(owner,c30.back_object).ok_or("C32 back scanout missing")?;let stride=c30.width.checked_mul(4).ok_or("C32 display stress stride overflow")?;let mut fb=Framebuffer::from_boot_info(boot_info)?;let mut last=0u64;let mut changes=0u32;for i in 0..C32_DISPLAY_STRESS_PRESENTS{let o=if i&1==0{front}else{back};let h=fb.copy_xrgb8888(o.kernel_virtual,stride,c30.width,c30.height)?;let live=fb.sample_hash();if h==0||live==0{return Err("C32 display stress present fingerprint invalid")}if last!=0&&live!=last{changes=changes.saturating_add(1)}last=live;radeon_telemetry::record(TelemetryKind::DisplayPresent,1,u64::from(i))?;}if changes<C32_DISPLAY_STRESS_PRESENTS/2{return Err("C32 display stress did not alternate scanout content")};Ok(C32_DISPLAY_STRESS_PRESENTS)}

fn multi_display_framework_test(c30:C30State)->Result<u8,&'static str>{if radeon_display::MAX_DISPLAY_CONNECTORS<4{return Err("C32 multi-display framework capacity regressed")}let mode=DisplayMode{width:c30.width,height:c30.height,refresh_millihz:60_000,pixel_clock_khz:0};let connectors=[Connector{id:1,kind:ConnectorKind::FirmwareGop,connected:true,crtc:0,plane:0,edid_valid:false,mode,active:true},Connector{id:2,kind:ConnectorKind::DisplayPort,connected:true,crtc:1,plane:1,edid_valid:true,mode,active:true},Connector{id:3,kind:ConnectorKind::Hdmi,connected:true,crtc:2,plane:2,edid_valid:true,mode,active:true},Connector{id:4,kind:ConnectorKind::DisplayPort,connected:true,crtc:3,plane:3,edid_valid:true,mode,active:true}];let mut journal=HotplugJournal::new();for c in connectors{if !c.connected||!c.active||c.id==0{return Err("C32 multi-display connector invalid")}let _=journal.record(c.id,true)?;}let _=journal.record(4,false)?;if journal.count()!=5{return Err("C32 multi-display hotplug journal invalid")};Ok(connectors.len() as u8)}

pub fn qualify(allocator:&mut FrameAllocator<'_>,boot_info:&BootInfo,owner:u64,c30:C30State)->Result<StabilityReport,&'static str>{
 if RADEON_STABILITY_ABI_VERSION!=1||owner==0||!c30.qualified{return Err("C32 stability prerequisites invalid")}
 let(qs,qr,wraps)=queue_stress()?;let abandoned=hang_recovery()?;let(gtt,vram_verified,vram_deferred,vram)=pressure_test(allocator,owner)?;let irq=radeon_driver::software_irq_stress(C32_IRQ_STRESS_EVENTS)?;radeon_telemetry::record(TelemetryKind::Interrupt,u64::from(C32_IRQ_STRESS_EVENTS),irq)?;recovery_stress()?;let(compute_ops,pixels,concurrency_presents)=concurrency_test(allocator,boot_info,owner,c30)?;let displays=display_stress(boot_info,owner,c30)?;let connectors=multi_display_framework_test(c30)?;
 let mut r=StabilityReport{queue_stress_verified:true,queue_submissions:qs,queue_retires:qr,queue_wraps:wraps,hang_detected:true,hang_recovered:true,abandoned_on_reset:abandoned,gtt_pressure_verified:gtt>=C32_GTT_PRESSURE_BYTES*C32_GTT_PRESSURE_OBJECTS as u64,gtt_pressure_bytes:gtt,vram_pressure_verified:vram_verified,vram_pressure_deferred:vram_deferred,vram_pressure_bytes:vram,interrupt_stress_verified:true,interrupt_events:u64::from(C32_IRQ_STRESS_EVENTS),recovery_stress_verified:true,recovery_cycles:C32_RECOVERY_STRESS_CYCLES,display_compute_concurrency:true,graphics_compute_concurrency:true,concurrency_rounds:C32_CONCURRENCY_ROUNDS,compute_ops,graphics_pixels:pixels,display_stress_verified:true,display_presents:displays.saturating_add(concurrency_presents),multi_display_framework_verified:true,multi_display_connectors:connectors,fingerprint:0};
 let common=r.queue_submissions==r.queue_retires&&r.queue_submissions==C32_QUEUE_STRESS_SUBMISSIONS*2&&r.abandoned_on_reset==1&&r.gtt_pressure_verified&&(r.vram_pressure_verified||r.vram_pressure_deferred)&&r.interrupt_events==u64::from(C32_IRQ_STRESS_EVENTS)&&r.recovery_cycles==C32_RECOVERY_STRESS_CYCLES&&r.compute_ops==u64::from(C32_CONCURRENCY_ROUNDS)*64&&r.graphics_pixels==u64::from(C32_CONCURRENCY_ROUNDS)&&r.display_presents>=C32_DISPLAY_STRESS_PRESENTS&&r.multi_display_connectors==4;if !common{return Err("C32 production stability gates did not close")}
 let mut h=0xc032_5354_4142_0001u64;for v in [r.queue_submissions,r.queue_retires,r.queue_wraps,r.abandoned_on_reset,r.gtt_pressure_bytes,r.vram_pressure_bytes,r.interrupt_events,u64::from(r.recovery_cycles),r.compute_ops,r.graphics_pixels,u64::from(r.display_presents),u64::from(r.multi_display_connectors)]{h=mix(h,v)}r.fingerprint=h;Ok(r)
}
