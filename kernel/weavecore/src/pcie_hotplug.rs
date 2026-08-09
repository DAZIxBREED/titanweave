//! PCIe hot-plug slot state, debouncing, surprise removal and reset recovery.
use crate::{device::DeviceId,pci_address::PciAddress};
pub const MAX_HOTPLUG_SLOTS:usize=64;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum SlotState{Empty,PresentDebounce,Present,PoweringOff,SurpriseRemoved,Failed}
#[derive(Clone,Copy,Debug)]pub struct Slot{pub bridge:PciAddress,pub number:u8,pub device:Option<DeviceId>,pub state:SlotState,pub generation:u32,pub last_change:u64}impl Slot{pub const EMPTY:Self=Self{bridge:PciAddress{segment:0,bus:0,device:0,function:0},number:0,device:None,state:SlotState::Empty,generation:0,last_change:0};}
pub struct HotplugController{slots:[Slot;MAX_HOTPLUG_SLOTS],count:usize,debounce_ticks:u64}
impl HotplugController{
 pub const fn new()->Self{Self{slots:[Slot::EMPTY;MAX_HOTPLUG_SLOTS],count:0,debounce_ticks:10}}
 pub fn register_slot(&mut self,bridge:PciAddress,number:u8)->Result<(),&'static str>{if self.slots[..self.count].iter().any(|s|s.bridge==bridge&&s.number==number){return Err("duplicate PCIe slot")}if self.count>=MAX_HOTPLUG_SLOTS{return Err("hot-plug slot table full")}self.slots[self.count]=Slot{bridge,number,device:None,state:SlotState::Empty,generation:1,last_change:0};self.count+=1;Ok(())}
 pub fn presence_change(&mut self,bridge:PciAddress,number:u8,present:bool,now:u64)->Result<(),&'static str>{let s=self.slots[..self.count].iter_mut().find(|s|s.bridge==bridge&&s.number==number).ok_or("slot not found")?;s.last_change=now;s.generation=s.generation.wrapping_add(1).max(1);s.state=if present{SlotState::PresentDebounce}else if s.device.is_some(){SlotState::SurpriseRemoved}else{SlotState::Empty};Ok(())}
 pub fn poll(&mut self,now:u64,mut arrived:impl FnMut(PciAddress,u8,u32),mut removed:impl FnMut(DeviceId,u32))->Result<(),&'static str>{for s in &mut self.slots[..self.count]{match s.state{SlotState::PresentDebounce if now.saturating_sub(s.last_change)>=self.debounce_ticks=>{s.state=SlotState::Present;arrived(s.bridge,s.number,s.generation)},SlotState::SurpriseRemoved=>{if let Some(d)=s.device.take(){removed(d,s.generation)}s.state=SlotState::Empty},_=>{}}}Ok(())}
 pub fn bind_device(&mut self,bridge:PciAddress,number:u8,device:DeviceId,generation:u32)->Result<(),&'static str>{let s=self.slots[..self.count].iter_mut().find(|s|s.bridge==bridge&&s.number==number).ok_or("slot not found")?;if s.generation!=generation||s.state!=SlotState::Present{return Err("stale hot-plug generation")}s.device=Some(device);Ok(())}
}
