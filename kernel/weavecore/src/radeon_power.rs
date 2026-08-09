//! K14.C32 bounded Radeon power-state policy.
//!
//! C32 freezes a driver-visible power state machine and idle/quiesce behavior.
//! It deliberately does not program unqualified SMU/PM MMIO or clocks.

use crate::{radeon_telemetry::{self,TelemetryKind},sync::SpinLock};

pub const RADEON_POWER_ABI_VERSION:u32=1;
pub const RADEON_C32_PHYSICAL_SMU_PROGRAMMING:bool=false;
#[repr(u8)]#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum PowerState{Boot=0,Active=1,Idle=2,Quiesced=3,Fault=4}
#[derive(Clone,Copy,Debug)]pub struct PowerSnapshot{pub state:PowerState,pub transitions:u64,pub rejected:u64,pub physical_smu_programmed:bool,pub fingerprint:u64}
impl PowerSnapshot{pub const EMPTY:Self=Self{state:PowerState::Boot,transitions:0,rejected:0,physical_smu_programmed:false,fingerprint:0};}
pub struct PowerMachine{state:PowerState,transitions:u64,rejected:u64}
impl PowerMachine{
 pub const fn new()->Self{Self{state:PowerState::Boot,transitions:0,rejected:0}}
 pub fn transition(&mut self,to:PowerState)->Result<(),&'static str>{let allowed=matches!((self.state,to),(PowerState::Boot,PowerState::Active)|(PowerState::Active,PowerState::Idle)|(PowerState::Idle,PowerState::Active)|(PowerState::Active,PowerState::Quiesced)|(PowerState::Idle,PowerState::Quiesced)|(PowerState::Quiesced,PowerState::Active)|(PowerState::Active,PowerState::Fault)|(PowerState::Idle,PowerState::Fault)|(PowerState::Quiesced,PowerState::Fault)|(PowerState::Fault,PowerState::Quiesced));if !allowed{self.rejected=self.rejected.saturating_add(1);return Err("C32 illegal Radeon power transition")}self.state=to;self.transitions=self.transitions.saturating_add(1);Ok(())}
 pub fn snapshot(&self)->PowerSnapshot{let fp=0xc032_504f_5745_0001u64^self.transitions^(self.rejected<<16)^((self.state as u64)<<32);PowerSnapshot{state:self.state,transitions:self.transitions,rejected:self.rejected,physical_smu_programmed:false,fingerprint:fp}}
}
static POWER:SpinLock<PowerMachine>=SpinLock::new(PowerMachine::new());
pub fn initialize_runtime()->Result<PowerSnapshot,&'static str>{if RADEON_POWER_ABI_VERSION!=1||RADEON_C32_PHYSICAL_SMU_PROGRAMMING{return Err("C32 power policy invalid")}let mut p=POWER.lock();if p.state==PowerState::Boot{p.transition(PowerState::Active)?;radeon_telemetry::record(TelemetryKind::PowerTransition,1,PowerState::Active as u64)?}p.transition(PowerState::Idle)?;radeon_telemetry::record(TelemetryKind::PowerTransition,1,PowerState::Idle as u64)?;p.transition(PowerState::Active)?;radeon_telemetry::record(TelemetryKind::PowerTransition,1,PowerState::Active as u64)?;Ok(p.snapshot())}
pub fn snapshot()->PowerSnapshot{POWER.lock().snapshot()}
pub fn self_test()->Result<u64,&'static str>{let mut p=PowerMachine::new();p.transition(PowerState::Active)?;p.transition(PowerState::Idle)?;p.transition(PowerState::Quiesced)?;p.transition(PowerState::Active)?;if p.transition(PowerState::Boot).is_ok(){return Err("C32 power machine accepted reverse-to-boot transition")}let s=p.snapshot();if s.state!=PowerState::Active||s.transitions!=4||s.rejected!=1||s.physical_smu_programmed{return Err("C32 power self-test failed")}Ok(s.fingerprint)}
