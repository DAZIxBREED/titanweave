//! K14.C28 operational Radeon watchdog and resource-safe software recovery.
//!
//! The recovery path is executable today: it registers the actual retained
//! Radeon driver with Titanweave's watchdog, can quiesce/fault/coordinate/resume
//! the live C27 driver lifecycle, masks its interrupt route during recovery, and
//! reclaims caller-owned C28 GTT/VRAM objects.  C28 does not claim a physical
//! ASIC reset; production hardware reset policy remains a later stability gate.

use crate::{
 device::DeviceId,
 driver_watchdog::{DriverWatchdog,WatchdogAction},
 kernel_runtime,
 memory::FrameAllocator,
 radeon_driver,
 radeon_memory,
 sync::SpinLock,
};

pub const RADEON_RECOVERY_ABI_VERSION:u32=1;
pub const RADEON_WATCHDOG_TIMEOUT_TICKS:u64=500;
pub const RADEON_WATCHDOG_RESTART_LIMIT:u8=2;
pub const RADEON_C28_PHYSICAL_ASIC_RESET_PERFORMED:bool=false;

#[derive(Clone,Copy,Debug)]
pub struct RadeonRecoveryState{
 pub model_verified:bool,pub watchdog_registered:bool,pub watchdog_actions_verified:bool,pub lifecycle_recovery_verified:bool,
 pub interrupt_fencing_ready:bool,pub interrupt_route_active:bool,pub resource_reclaim_ready:bool,pub recoveries:u32,pub quarantines:u32,
 pub physical_asic_reset_performed:bool,pub fingerprint:u64,
}
impl RadeonRecoveryState{pub const EMPTY:Self=Self{model_verified:false,watchdog_registered:false,watchdog_actions_verified:false,lifecycle_recovery_verified:false,interrupt_fencing_ready:false,interrupt_route_active:false,resource_reclaim_ready:false,recoveries:0,quarantines:0,physical_asic_reset_performed:false,fingerprint:0};}
static STATE:SpinLock<RadeonRecoveryState>=SpinLock::new(RadeonRecoveryState::EMPTY);

fn watchdog_self_test()->Result<(),&'static str>{
 let mut w=DriverWatchdog::new();let d=DeviceId(0xc028);w.register(0x28,d,10,5,1)?;
 if w.evaluate(0x28,d,14)?!=WatchdogAction::None{return Err("Radeon watchdog fired before timeout")}
 if w.evaluate(0x28,d,15)?!=WatchdogAction::Ping{return Err("Radeon watchdog did not ping on first timeout")}
 if w.evaluate(0x28,d,16)?!=WatchdogAction::Restart{return Err("Radeon watchdog did not restart after unanswered ping")}
 if w.evaluate(0x28,d,21)?!=WatchdogAction::Ping{return Err("Radeon watchdog second timeout did not ping")}
 if w.evaluate(0x28,d,22)?!=WatchdogAction::Quarantine{return Err("Radeon watchdog restart limit did not quarantine")}
 Ok(())
}
fn lifecycle_self_test()->Result<(),&'static str>{let mut m=radeon_driver::CoreMachine::new();m.claim()?;m.mmio_ready()?;m.online()?;m.quiesce()?;m.fault(0xc028)?;m.coordinate_reset()?;m.online()?;if m.phase!=radeon_driver::CorePhase::Online||m.fault_count!=1||m.reset_epoch!=1{return Err("Radeon C28 recovery lifecycle self-test failed")}Ok(())}
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}

pub fn initialize()->Result<RadeonRecoveryState,&'static str>{
 if RADEON_RECOVERY_ABI_VERSION!=1||RADEON_C28_PHYSICAL_ASIC_RESET_PERFORMED{return Err("Radeon C28 recovery policy constants invalid")}
 watchdog_self_test()?;lifecycle_self_test()?;let d=radeon_driver::state();let mut s=RadeonRecoveryState{model_verified:true,watchdog_actions_verified:true,lifecycle_recovery_verified:true,interrupt_fencing_ready:true,resource_reclaim_ready:true,..RadeonRecoveryState::EMPTY};
 if !d.hardware_deferred&&d.forge_device.0!=0&&d.forge_driver!=0{
  kernel_runtime::with_runtime(|r|r.watchdog.register(d.forge_driver,d.forge_device,0,RADEON_WATCHDOG_TIMEOUT_TICKS,RADEON_WATCHDOG_RESTART_LIMIT))?;s.watchdog_registered=true;
  s.interrupt_route_active=radeon_driver::activate_recovery_interrupt_route()?;
 }
 let mut fp=0xc28c_5245_434f_5601u64;for v in [d.fingerprint,d.forge_device.0,d.forge_driver,RADEON_WATCHDOG_TIMEOUT_TICKS,u64::from(RADEON_WATCHDOG_RESTART_LIMIT)]{fp=mix(fp,v)}s.fingerprint=fp;*STATE.lock()=s;Ok(s)
}

/// Execute one bounded software recovery transaction for the live Radeon.
/// `owned_objects` are reclaimed before the driver is returned online.
pub fn recover(allocator:&mut FrameAllocator<'_>,owner:u64,owned_objects:&[u64],fault_code:u32)->Result<(),&'static str>{
 if fault_code==0{return Err("Radeon recovery requires nonzero fault code")}
 let d=radeon_driver::state();if d.hardware_deferred{return Err("no physical Radeon exists for live recovery")}
 radeon_driver::quiesce_for_recovery()?;
 if d.irq_route_registered{radeon_driver::mask_recovery_interrupt_route()?;}
 radeon_driver::record_recovery_fault(fault_code)?;
 for &id in owned_objects{if id==0{continue}if let Some(o)=radeon_memory::object(owner,id){if o.pinned{radeon_memory::pin(owner,id,false)?}radeon_memory::free(allocator,owner,id)?;}}
 radeon_driver::coordinate_recovery()?;radeon_driver::resume_after_recovery()?;let _=radeon_driver::activate_recovery_interrupt_route()?;
 let mut s=STATE.lock();s.recoveries=s.recoveries.saturating_add(1);Ok(())
}

pub fn heartbeat(now:u64)->Result<(),&'static str>{let d=radeon_driver::state();if d.hardware_deferred{return Ok(())}kernel_runtime::with_runtime(|r|r.watchdog.heartbeat(d.forge_driver,d.forge_device,now))}
pub fn state()->RadeonRecoveryState{*STATE.lock()}
