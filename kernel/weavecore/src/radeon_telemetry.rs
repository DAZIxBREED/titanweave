//! K14.C32 production telemetry, event journal, and diagnostic counters.
//!
//! The journal is intentionally bounded and allocation-free so that the final
//! Radeon milestone can report stress/recovery activity even when memory is
//! under pressure.  These counters describe Titanweave driver activity; they do
//! not claim access to unqualified physical Radeon performance-counter MMIO.

use crate::sync::SpinLock;

pub const RADEON_TELEMETRY_ABI_VERSION:u32=1;
pub const C32_TELEMETRY_EVENTS:usize=32;
pub const RADEON_C32_PHYSICAL_PERF_COUNTER_MMIO:bool=false;

#[repr(u8)]
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TelemetryKind{QueueSubmit=1,QueueRetire=2,MemoryAllocate=3,MemoryFree=4,Interrupt=5,HangDetected=6,HangRecovered=7,DisplayPresent=8,RecoveryCycle=9,PowerTransition=10,Error=11}

#[derive(Clone,Copy,Debug)]
pub struct TelemetryEvent{pub sequence:u64,pub kind:TelemetryKind,pub value:u64,pub aux:u64}
impl TelemetryEvent{pub const EMPTY:Self=Self{sequence:0,kind:TelemetryKind::Error,value:0,aux:0};}

#[derive(Clone,Copy,Debug)]
pub struct TelemetrySnapshot{
 pub sequence:u64,pub events_recorded:u64,pub queue_submits:u64,pub queue_retires:u64,pub memory_allocations:u64,pub memory_frees:u64,
 pub interrupt_events:u64,pub hangs_detected:u64,pub hangs_recovered:u64,pub display_presents:u64,pub recovery_cycles:u64,pub power_transitions:u64,pub errors:u64,
 pub compute_operations:u64,pub graphics_pixels:u64,pub bytes_touched:u64,pub fingerprint:u64,
}
impl TelemetrySnapshot{pub const EMPTY:Self=Self{sequence:0,events_recorded:0,queue_submits:0,queue_retires:0,memory_allocations:0,memory_frees:0,interrupt_events:0,hangs_detected:0,hangs_recovered:0,display_presents:0,recovery_cycles:0,power_transitions:0,errors:0,compute_operations:0,graphics_pixels:0,bytes_touched:0,fingerprint:0};}

pub struct Telemetry{events:[TelemetryEvent;C32_TELEMETRY_EVENTS],next:u64,count:u64,s:TelemetrySnapshot}
impl Telemetry{
 pub const fn new()->Self{Self{events:[TelemetryEvent::EMPTY;C32_TELEMETRY_EVENTS],next:1,count:0,s:TelemetrySnapshot::EMPTY}}
 pub fn record(&mut self,kind:TelemetryKind,value:u64,aux:u64)->Result<(),&'static str>{
  if self.next==0{return Err("C32 telemetry sequence exhausted")}
  let e=TelemetryEvent{sequence:self.next,kind,value,aux};
  self.events[((self.next-1) as usize)%C32_TELEMETRY_EVENTS]=e;
  self.next=self.next.checked_add(1).ok_or("C32 telemetry sequence exhausted")?;self.count=self.count.saturating_add(1);
  match kind{TelemetryKind::QueueSubmit=>self.s.queue_submits=self.s.queue_submits.saturating_add(value.max(1)),TelemetryKind::QueueRetire=>self.s.queue_retires=self.s.queue_retires.saturating_add(value.max(1)),TelemetryKind::MemoryAllocate=>{self.s.memory_allocations=self.s.memory_allocations.saturating_add(1);self.s.bytes_touched=self.s.bytes_touched.saturating_add(value)},TelemetryKind::MemoryFree=>self.s.memory_frees=self.s.memory_frees.saturating_add(1),TelemetryKind::Interrupt=>self.s.interrupt_events=self.s.interrupt_events.saturating_add(value.max(1)),TelemetryKind::HangDetected=>self.s.hangs_detected=self.s.hangs_detected.saturating_add(1),TelemetryKind::HangRecovered=>self.s.hangs_recovered=self.s.hangs_recovered.saturating_add(1),TelemetryKind::DisplayPresent=>self.s.display_presents=self.s.display_presents.saturating_add(value.max(1)),TelemetryKind::RecoveryCycle=>self.s.recovery_cycles=self.s.recovery_cycles.saturating_add(value.max(1)),TelemetryKind::PowerTransition=>self.s.power_transitions=self.s.power_transitions.saturating_add(value.max(1)),TelemetryKind::Error=>self.s.errors=self.s.errors.saturating_add(1)}
  Ok(())
 }
 pub fn add_compute_ops(&mut self,n:u64){self.s.compute_operations=self.s.compute_operations.saturating_add(n)}
 pub fn add_graphics_pixels(&mut self,n:u64){self.s.graphics_pixels=self.s.graphics_pixels.saturating_add(n)}
 pub fn snapshot(&self)->TelemetrySnapshot{let mut s=self.s;s.sequence=self.next.saturating_sub(1);s.events_recorded=self.count;let mut h=0xc032_5445_4c45_0001u64;for v in [s.sequence,s.events_recorded,s.queue_submits,s.queue_retires,s.memory_allocations,s.memory_frees,s.interrupt_events,s.hangs_detected,s.hangs_recovered,s.display_presents,s.recovery_cycles,s.power_transitions,s.errors,s.compute_operations,s.graphics_pixels,s.bytes_touched]{h^=v;h=h.wrapping_mul(0x100000001b3)}s.fingerprint=h;s}
 pub fn latest(&self)->TelemetryEvent{if self.count==0{return TelemetryEvent::EMPTY}self.events[((self.next-2) as usize)%C32_TELEMETRY_EVENTS]}
}
static TELEMETRY:SpinLock<Telemetry>=SpinLock::new(Telemetry::new());

pub fn record(kind:TelemetryKind,value:u64,aux:u64)->Result<(),&'static str>{TELEMETRY.lock().record(kind,value,aux)}
pub fn add_compute_ops(n:u64){TELEMETRY.lock().add_compute_ops(n)}
pub fn add_graphics_pixels(n:u64){TELEMETRY.lock().add_graphics_pixels(n)}
pub fn snapshot()->TelemetrySnapshot{TELEMETRY.lock().snapshot()}

pub fn self_test()->Result<u64,&'static str>{
 if RADEON_TELEMETRY_ABI_VERSION!=1||C32_TELEMETRY_EVENTS!=32||RADEON_C32_PHYSICAL_PERF_COUNTER_MMIO{return Err("C32 telemetry policy invalid")}
 let mut t=Telemetry::new();t.record(TelemetryKind::QueueSubmit,2,0)?;t.record(TelemetryKind::QueueRetire,2,0)?;t.record(TelemetryKind::HangDetected,1,7)?;t.record(TelemetryKind::HangRecovered,1,7)?;t.add_compute_ops(64);t.add_graphics_pixels(128);let s=t.snapshot();
 if s.events_recorded!=4||s.queue_submits!=2||s.queue_retires!=2||s.hangs_detected!=1||s.hangs_recovered!=1||s.compute_operations!=64||s.graphics_pixels!=128||s.errors!=0||t.latest().kind!=TelemetryKind::HangRecovered{return Err("C32 telemetry self-test failed")}
 Ok(s.fingerprint)
}
