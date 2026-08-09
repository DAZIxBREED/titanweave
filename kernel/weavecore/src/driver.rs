//! K11 signed driver registry, matching, binding, and recovery.
use crate::{device::{Device,DeviceId,DeviceRegistry,DeviceState},trust::TrustLevel};

pub const MAX_DRIVERS: usize = 128;
pub const MAX_DRIVER_MATCHES: usize = 16;
#[derive(Clone,Copy,Debug,PartialEq,Eq)] pub enum DriverIsolation { Kernel, User }
#[derive(Clone,Copy,Debug,PartialEq,Eq)] pub enum DriverState { Registered, Active, Quarantined, Disabled }
#[derive(Clone,Copy,Debug)] pub struct DriverMatch { pub vendor_id: Option<u16>, pub product_id: Option<u16>, pub class_code: Option<u8>, pub subclass: Option<u8>, pub programming_interface: Option<u8> }
impl DriverMatch { pub const NONE:Self=Self{vendor_id:None,product_id:None,class_code:None,subclass:None,programming_interface:None}; pub fn matches(&self,d:&Device)->bool { self.vendor_id.map_or(true,|x|x==d.vendor_id)&&self.product_id.map_or(true,|x|x==d.product_id)&&self.class_code.map_or(true,|x|x==d.class_code)&&self.subclass.map_or(true,|x|x==d.subclass)&&self.programming_interface.map_or(true,|x|x==d.programming_interface) } }
#[derive(Clone,Copy,Debug)] pub struct DriverDescriptor { pub id:u64,pub name:[u8;32],pub version:u64,pub signer:TrustLevel,pub isolation:DriverIsolation,pub state:DriverState,pub matches:[DriverMatch;MAX_DRIVER_MATCHES],pub match_count:usize,pub restart_limit:u8,pub crashes:u8 }
impl DriverDescriptor { pub const EMPTY:Self=Self{id:0,name:[0;32],version:0,signer:TrustLevel::Untrusted,isolation:DriverIsolation::User,state:DriverState::Disabled,matches:[DriverMatch::NONE;MAX_DRIVER_MATCHES],match_count:0,restart_limit:0,crashes:0}; }
pub struct DriverRegistry { drivers:[DriverDescriptor;MAX_DRIVERS],next_id:u64 }
impl DriverRegistry {
 pub const fn new()->Self{Self{drivers:[DriverDescriptor::EMPTY;MAX_DRIVERS],next_id:1}}
 pub fn register(&mut self,mut d:DriverDescriptor)->Result<u64,&'static str>{if d.match_count==0||d.match_count>MAX_DRIVER_MATCHES{return Err("driver has no valid match rules")}if matches!(d.isolation,DriverIsolation::Kernel)&&!matches!(d.signer,TrustLevel::System|TrustLevel::Platform){return Err("kernel driver requires system/platform signer")}let s=self.drivers.iter_mut().find(|x|x.id==0||x.state==DriverState::Disabled).ok_or("driver registry full")?;d.id=self.next_id;self.next_id=self.next_id.checked_add(1).ok_or("driver id exhausted")?;d.state=DriverState::Registered;*s=d;Ok(d.id)}
 pub fn best_match(&self,dev:&Device)->Option<&DriverDescriptor>{self.drivers.iter().filter(|d|matches!(d.state,DriverState::Registered|DriverState::Active)&&d.matches[..d.match_count].iter().any(|m|m.matches(dev))).max_by_key(|d|{let m=&d.matches[..d.match_count];m.iter().map(|x|x.vendor_id.is_some()as u8+x.product_id.is_some()as u8+x.class_code.is_some()as u8+x.subclass.is_some()as u8+x.programming_interface.is_some()as u8).max().unwrap_or(0)})}
 pub fn bind_best(&mut self,devices:&mut DeviceRegistry,id:DeviceId)->Result<u64,&'static str>{let driver_id={let dev=devices.get(id).ok_or("device not found")?;self.best_match(dev).ok_or("no matching driver")?.id};let dev=devices.get_mut(id).ok_or("device disappeared")?;dev.driver_id=Some(driver_id);dev.state=DeviceState::Bound;if let Some(d)=self.drivers.iter_mut().find(|d|d.id==driver_id){d.state=DriverState::Active;}Ok(driver_id)}
 pub fn report_crash(&mut self,driver_id:u64,devices:&mut DeviceRegistry)->Result<bool,&'static str>{let d=self.drivers.iter_mut().find(|x|x.id==driver_id).ok_or("driver not found")?;d.crashes=d.crashes.saturating_add(1);let restart=d.crashes<=d.restart_limit;if !restart{d.state=DriverState::Quarantined;}for dev in devices.slots_mut(){if dev.driver_id==Some(driver_id){dev.driver_id=None;dev.state=if restart{DeviceState::Authorized}else{DeviceState::Failed};}}Ok(restart)}
}
impl DeviceRegistry { pub(crate) fn slots_mut(&mut self)->impl Iterator<Item=&mut Device>{self.slots.iter_mut().filter(|d|d.id.0!=0&&d.state!=DeviceState::Removed)} }
impl DriverRegistry {
 pub fn get(&self,id:u64)->Option<&DriverDescriptor>{self.drivers.iter().find(|d|d.id==id&&!matches!(d.state,DriverState::Disabled))}
 pub fn get_mut(&mut self,id:u64)->Option<&mut DriverDescriptor>{self.drivers.iter_mut().find(|d|d.id==id&&!matches!(d.state,DriverState::Disabled))}
 pub fn unbind(&mut self,devices:&mut DeviceRegistry,driver_id:u64,failed:bool)->usize{let mut n=0;for dev in devices.slots_mut(){if dev.driver_id==Some(driver_id){dev.driver_id=None;dev.state=if failed{DeviceState::Failed}else{DeviceState::Authorized};n+=1}}if let Some(d)=self.get_mut(driver_id){d.state=if failed{DriverState::Quarantined}else{DriverState::Registered};}n}
}
