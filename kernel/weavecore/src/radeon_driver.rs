//! K14.C27 operational Radeon driver-core lifecycle.
//!
//! This is not a placeholder backend.  It owns the retained ForgeBus Radeon,
//! validates the permanent reviewed-MMIO service, registers a real interrupt
//! route/handler (kept masked because hardware interrupt programming belongs to
//! C29), and provides executable lifecycle, fault and reset-coordination state.
//! C28/C29 extend this object rather than replacing it.

use core::sync::atomic::{AtomicU64,Ordering};
use crate::{
    device::DeviceId,
    kernel_runtime,
    radeon_mmio::MmioServiceState,
    radeon_resources::RadeonResourceState,
    sync::SpinLock,
};

pub const RADEON_DRIVER_CORE_ABI_VERSION:u32=1;
pub const RADEON_C27_HARDWARE_IRQ_ENABLE_ALLOWED:bool=false;
pub const RADEON_C27_FIRMWARE_UPLOAD_ALLOWED:bool=false;
pub const RADEON_C27_DMA_ENABLE_ALLOWED:bool=false;
pub const RADEON_C27_COMMAND_SUBMIT_ALLOWED:bool=false;

#[repr(u8)]
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CorePhase{Deferred=0,Claimed=1,MmioReady=2,Online=3,Quiesced=4,Faulted=5}

#[derive(Clone,Copy,Debug)]
pub struct CoreMachine{
 pub phase:CorePhase,pub generation:u32,pub fault_count:u32,pub reset_epoch:u32,pub last_error:u32,
}
impl CoreMachine{
 pub const fn new()->Self{Self{phase:CorePhase::Deferred,generation:1,fault_count:0,reset_epoch:0,last_error:0}}
 pub fn claim(&mut self)->Result<(),&'static str>{if self.phase!=CorePhase::Deferred{return Err("Radeon core claim from invalid phase")}self.phase=CorePhase::Claimed;Ok(())}
 pub fn mmio_ready(&mut self)->Result<(),&'static str>{if self.phase!=CorePhase::Claimed{return Err("Radeon core MMIO transition from invalid phase")}self.phase=CorePhase::MmioReady;Ok(())}
 pub fn online(&mut self)->Result<(),&'static str>{if self.phase!=CorePhase::MmioReady&&self.phase!=CorePhase::Quiesced{return Err("Radeon core online transition from invalid phase")}self.phase=CorePhase::Online;self.generation=self.generation.saturating_add(1);Ok(())}
 pub fn quiesce(&mut self)->Result<(),&'static str>{if self.phase!=CorePhase::Online{return Err("Radeon core quiesce from invalid phase")}self.phase=CorePhase::Quiesced;Ok(())}
 pub fn fault(&mut self,code:u32)->Result<(),&'static str>{if code==0{return Err("Radeon core fault requires nonzero code")}self.phase=CorePhase::Faulted;self.fault_count=self.fault_count.saturating_add(1);self.last_error=code;Ok(())}
 pub fn coordinate_reset(&mut self)->Result<(),&'static str>{if self.phase!=CorePhase::Faulted&&self.phase!=CorePhase::Quiesced{return Err("Radeon core reset coordination from invalid phase")}self.reset_epoch=self.reset_epoch.saturating_add(1);self.last_error=0;self.phase=CorePhase::MmioReady;Ok(())}
}

#[derive(Clone,Copy,Debug)]
pub struct RadeonDriverState{
 pub model_verified:bool,pub amd_present:bool,pub hardware_deferred:bool,pub phase:CorePhase,
 pub forge_device:DeviceId,pub forge_driver:u64,pub ownership_verified:bool,pub resources_verified:bool,
 pub mmio_service_verified:bool,pub irq_route_registered:bool,pub irq_vector:u8,pub irq_masked:bool,
 pub irq_hardware_enabled:bool,pub irq_dispatches:u64,pub error_machine_verified:bool,
 pub reset_coordinator_verified:bool,pub generation:u32,pub fault_count:u32,pub reset_epoch:u32,
 pub core_online:bool,pub fingerprint:u64,
}
impl RadeonDriverState{pub const EMPTY:Self=Self{model_verified:false,amd_present:false,hardware_deferred:true,
 phase:CorePhase::Deferred,forge_device:DeviceId(0),forge_driver:0,ownership_verified:false,resources_verified:false,
 mmio_service_verified:false,irq_route_registered:false,irq_vector:0,irq_masked:true,irq_hardware_enabled:false,
 irq_dispatches:0,error_machine_verified:false,reset_coordinator_verified:false,generation:0,fault_count:0,
 reset_epoch:0,core_online:false,fingerprint:0};}

static STATE:SpinLock<RadeonDriverState>=SpinLock::new(RadeonDriverState::EMPTY);
static IRQ_EVENTS:AtomicU64=AtomicU64::new(0);
static LAST_IRQ:SpinLock<(u8,DeviceId)>=SpinLock::new((0,DeviceId(0)));

fn irq_handler(vector:u8,device:DeviceId)->Result<(),&'static str>{
 if device.0==0{return Err("Radeon interrupt dispatched without device owner")}
 IRQ_EVENTS.fetch_add(1,Ordering::Relaxed);
 *LAST_IRQ.lock()=(vector,device);
 Ok(())
}

fn machine_self_test()->Result<(),&'static str>{
 let mut m=CoreMachine::new();m.claim()?;m.mmio_ready()?;m.online()?;m.quiesce()?;m.fault(0xc027)?;m.coordinate_reset()?;m.online()?;
 if m.phase!=CorePhase::Online||m.fault_count!=1||m.reset_epoch!=1||m.last_error!=0{return Err("Radeon driver lifecycle/reset self-test failed")}
 if m.claim().is_ok(){return Err("Radeon driver invalid transition was accepted")}
 Ok(())
}

fn interrupt_router_self_test()->Result<(),&'static str>{
 use crate::interrupt_router::InterruptRouter;
 let mut r=InterruptRouter::new();let d=DeviceId(0xc027);let route=r.allocate(d,0,false)?;
 r.register_handler(route.vector,d,irq_handler)?;
 if r.record_dispatch(route.vector).is_ok(){return Err("masked Radeon test IRQ dispatched")}
 r.enable(route.vector,d)?;let owner=r.record_dispatch(route.vector)?;
 let h=r.handler(route.vector,owner).ok_or("Radeon test IRQ handler missing")?;h(route.vector,owner)?;
 r.mask(route.vector,d)?;r.release(route.vector,d)?;
 if IRQ_EVENTS.load(Ordering::Relaxed)==0{return Err("Radeon IRQ handler self-test did not execute")}
 Ok(())
}
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}

pub fn initialize(resources:RadeonResourceState,mmio:MmioServiceState)->Result<RadeonDriverState,&'static str>{
 if RADEON_DRIVER_CORE_ABI_VERSION!=1||RADEON_C27_HARDWARE_IRQ_ENABLE_ALLOWED||RADEON_C27_FIRMWARE_UPLOAD_ALLOWED||RADEON_C27_DMA_ENABLE_ALLOWED||RADEON_C27_COMMAND_SUBMIT_ALLOWED{return Err("Radeon C27 driver-core policy constants invalid")}
 machine_self_test()?;interrupt_router_self_test()?;
 let mut s=RadeonDriverState{model_verified:true,amd_present:resources.amd_present,hardware_deferred:!resources.amd_present,
  error_machine_verified:true,reset_coordinator_verified:true,..RadeonDriverState::EMPTY};
 if !resources.amd_present{
  s.mmio_service_verified=mmio.policy_ready&&mmio.generic_write_rejected;
  s.resources_verified=resources.model_ready;
  s.fingerprint=mix(0xc27c_4452_5652_0001,mmio.fingerprint^resources.fingerprint);
  *STATE.lock()=s;return Ok(s)
 }
 if !resources.topology_verified||!resources.forge_owned{return Err("Radeon driver core lacks verified ForgeBus resource ownership")}
 if !mmio.policy_ready||!mmio.reg0_read_valid||!mmio.reg1_read_valid||!mmio.generic_write_rejected{return Err("Radeon driver core lacks verified reviewed-MMIO service")}
 let mut machine=CoreMachine::new();machine.claim()?;machine.mmio_ready()?;
 let route=kernel_runtime::with_runtime(|runtime|{
  let route=runtime.interrupts.allocate(resources.forge_device,0,false)?;
  runtime.interrupts.register_handler(route.vector,resources.forge_device,irq_handler)?;
  Ok::<_,&'static str>(route)
 })?;
 if !route.masked{return Err("Radeon C27 interrupt route must remain hardware-safe masked")}
 machine.online()?;
 s.phase=machine.phase;s.generation=machine.generation;s.fault_count=machine.fault_count;s.reset_epoch=machine.reset_epoch;
 s.forge_device=resources.forge_device;s.forge_driver=resources.forge_driver;s.ownership_verified=true;s.resources_verified=true;
 s.mmio_service_verified=true;s.irq_route_registered=true;s.irq_vector=route.vector;s.irq_masked=route.masked;s.irq_hardware_enabled=false;
 s.irq_dispatches=IRQ_EVENTS.load(Ordering::Relaxed);s.core_online=s.phase==CorePhase::Online;s.hardware_deferred=false;
 let mut fp=0xc27c_434f_5245_0001u64;for v in [resources.fingerprint,mmio.fingerprint,s.forge_device.0,s.forge_driver,u64::from(s.irq_vector),u64::from(s.generation)]{fp=mix(fp,v)}s.fingerprint=fp;
 *STATE.lock()=s;Ok(s)
}
pub fn state()->RadeonDriverState{*STATE.lock()}
pub fn irq_events()->u64{IRQ_EVENTS.load(Ordering::Relaxed)}
pub fn last_irq()->(u8,DeviceId){*LAST_IRQ.lock()}

fn core_from_state(s:RadeonDriverState)->CoreMachine{CoreMachine{phase:s.phase,generation:s.generation,fault_count:s.fault_count,reset_epoch:s.reset_epoch,last_error:0}}
fn store_core(machine:CoreMachine){let mut s=STATE.lock();s.phase=machine.phase;s.generation=machine.generation;s.fault_count=machine.fault_count;s.reset_epoch=machine.reset_epoch;s.core_online=machine.phase==CorePhase::Online;}

/// Quiesce the live C27 driver object before C28 recovery/resource teardown.
pub fn quiesce_for_recovery()->Result<(),&'static str>{let s=*STATE.lock();if s.hardware_deferred{return Ok(())}let mut m=core_from_state(s);m.quiesce()?;store_core(m);Ok(())}
/// Record a real driver-core fault transition.  This does not pretend the GPU
/// has been physically reset; it fences software ownership for recovery.
pub fn record_recovery_fault(code:u32)->Result<(),&'static str>{let s=*STATE.lock();if s.hardware_deferred{return Ok(())}let mut m=core_from_state(s);m.fault(code)?;store_core(m);Ok(())}
/// Advance the operational driver lifecycle through its recovery coordinator.
pub fn coordinate_recovery()->Result<(),&'static str>{let s=*STATE.lock();if s.hardware_deferred{return Ok(())}let mut m=core_from_state(s);m.coordinate_reset()?;store_core(m);Ok(())}
/// Return a recovered driver object to Online after its owned resources are safe.
pub fn resume_after_recovery()->Result<(),&'static str>{let s=*STATE.lock();if s.hardware_deferred{return Ok(())}let mut m=core_from_state(s);m.online()?;store_core(m);Ok(())}

/// Activate Titanweave's owned software interrupt route for recovery-era
/// dispatch. This does not program Radeon IH/MSI hardware and therefore does
/// not claim C29's physical interrupt/DMA authority.
pub fn activate_recovery_interrupt_route()->Result<bool,&'static str>{
 let snapshot=*STATE.lock();if snapshot.hardware_deferred{return Ok(false)}
 if !snapshot.irq_route_registered{return Err("Radeon interrupt route is not registered")}
 kernel_runtime::with_runtime(|r|r.interrupts.enable(snapshot.irq_vector,snapshot.forge_device))?;
 let mut s=STATE.lock();s.irq_masked=false;Ok(true)
}
pub fn mask_recovery_interrupt_route()->Result<(),&'static str>{
 let snapshot=*STATE.lock();if snapshot.hardware_deferred{return Ok(())}
 if snapshot.irq_route_registered{kernel_runtime::with_runtime(|r|r.interrupts.mask(snapshot.irq_vector,snapshot.forge_device))?;}
 STATE.lock().irq_masked=true;Ok(())
}

/// C32 bounded software interrupt-handler stress. This exercises the actual
/// Radeon driver interrupt handler accounting without programming Radeon IH/MSI
/// hardware. The live hardware route is neither enabled nor reconfigured here.
pub fn software_irq_stress(rounds:u32)->Result<u64,&'static str>{
 if rounds<64||rounds>4096{return Err("C32 Radeon software IRQ stress rounds outside bound")}
 let snapshot=state();let device=if snapshot.forge_device.0!=0{snapshot.forge_device}else{DeviceId(0xc032)};let vector=if snapshot.irq_vector!=0{snapshot.irq_vector}else{0xf0};let before=IRQ_EVENTS.load(Ordering::Relaxed);
 for _ in 0..rounds{irq_handler(vector,device)?}
 let after=IRQ_EVENTS.load(Ordering::Relaxed);if after.saturating_sub(before)!=u64::from(rounds){return Err("C32 Radeon software IRQ stress accounting mismatch")}
 Ok(after)
}
