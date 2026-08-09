//! K11.0 driver heartbeat, timeout, and restart supervision.
use crate::device::DeviceId;
pub const MAX_WATCHED_DRIVERS:usize=128;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum WatchdogAction{None,Ping,Restart,Quarantine}
#[derive(Clone,Copy,Debug)]pub struct WatchRecord{pub driver_id:u64,pub device:DeviceId,pub last_heartbeat:u64,pub timeout_ticks:u64,pub restart_count:u8,pub restart_limit:u8,pub active:bool,pub pinged:bool}
impl WatchRecord{pub const EMPTY:Self=Self{driver_id:0,device:DeviceId(0),last_heartbeat:0,timeout_ticks:0,restart_count:0,restart_limit:0,active:false,pinged:false};}
pub struct DriverWatchdog{records:[WatchRecord;MAX_WATCHED_DRIVERS]}
impl DriverWatchdog{
 pub const fn new()->Self{Self{records:[WatchRecord::EMPTY;MAX_WATCHED_DRIVERS]}}
 pub fn register(&mut self,driver_id:u64,device:DeviceId,now:u64,timeout:u64,restart_limit:u8)->Result<(),&'static str>{if driver_id==0||device.0==0||timeout==0{return Err("invalid watchdog registration")}let s=self.records.iter_mut().find(|r|!r.active).ok_or("driver watchdog full")?;*s=WatchRecord{driver_id,device,last_heartbeat:now,timeout_ticks:timeout,restart_count:0,restart_limit,active:true,pinged:false};Ok(())}
 pub fn heartbeat(&mut self,driver_id:u64,device:DeviceId,now:u64)->Result<(),&'static str>{let r=self.records.iter_mut().find(|r|r.active&&r.driver_id==driver_id&&r.device==device).ok_or("driver not watched")?;r.last_heartbeat=now;r.pinged=false;Ok(())}
 pub fn evaluate(&mut self,driver_id:u64,device:DeviceId,now:u64)->Result<WatchdogAction,&'static str>{let r=self.records.iter_mut().find(|r|r.active&&r.driver_id==driver_id&&r.device==device).ok_or("driver not watched")?;let elapsed=now.saturating_sub(r.last_heartbeat);if elapsed<r.timeout_ticks{return Ok(WatchdogAction::None)}if !r.pinged{r.pinged=true;return Ok(WatchdogAction::Ping)}if r.restart_count<r.restart_limit{r.restart_count+=1;r.last_heartbeat=now;r.pinged=false;return Ok(WatchdogAction::Restart)}r.active=false;Ok(WatchdogAction::Quarantine)}
 pub fn unregister(&mut self,driver_id:u64,device:DeviceId)->Result<(),&'static str>{let r=self.records.iter_mut().find(|r|r.active&&r.driver_id==driver_id&&r.device==device).ok_or("driver not watched")?;r.active=false;Ok(())}
}
