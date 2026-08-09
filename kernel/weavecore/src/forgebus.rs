//! K11 ForgeBus persistent integration, recovery, and interrupt dispatch.
use crate::{
 device::{BusType,Device,DeviceId,DeviceRegistry,DeviceState,Resource},
 dma::{DmaDirection,DmaManager,DmaMapping},
 driver::{DriverDescriptor,DriverIsolation,DriverMatch,DriverRegistry,DriverState},
 driver_watchdog::WatchdogAction,
 hotplug::{HotplugJournal,HotplugKind},
 kernel_runtime,
 memory::FrameAllocator,
 pci,serial,
 trust::TrustLevel,
 sync::SpinLock,
};

pub struct ForgeBusReport{pub pci_functions:usize,pub devices:usize,pub drivers:usize,pub bound:usize}
pub struct ForgeBusRuntime{pub initialized:bool,pub devices:DeviceRegistry,pub drivers:DriverRegistry,pub dma:DmaManager,pub hotplug:HotplugJournal}
impl ForgeBusRuntime{pub const fn new()->Self{Self{initialized:false,devices:DeviceRegistry::new(),drivers:DriverRegistry::new(),dma:DmaManager::new(),hotplug:HotplugJournal::new()}}}
static BUS:SpinLock<ForgeBusRuntime>=SpinLock::new(ForgeBusRuntime::new());
fn with_bus<R>(f:impl FnOnce(&mut ForgeBusRuntime)->R)->R{let mut bus=BUS.lock();f(&mut bus)}
fn name(s:&[u8])->[u8;32]{let mut n=[0;32];let l=core::cmp::min(s.len(),32);n[..l].copy_from_slice(&s[..l]);n}

pub fn initialize()->Result<ForgeBusReport,&'static str>{with_bus(|bus|{
 if bus.initialized{return Err("ForgeBus already initialized")}
 let nvme=DriverDescriptor{id:0,name:name(b"titan-nvme"),version:1,signer:TrustLevel::System,isolation:DriverIsolation::Kernel,state:DriverState::Registered,matches:[DriverMatch{vendor_id:None,product_id:None,class_code:Some(0x01),subclass:Some(0x08),programming_interface:Some(0x02)};16],match_count:1,restart_limit:1,crashes:0};bus.drivers.register(nvme)?;
 let hid=DriverDescriptor{id:0,name:name(b"titan-usb-hid"),version:1,signer:TrustLevel::System,isolation:DriverIsolation::User,state:DriverState::Registered,matches:[DriverMatch{vendor_id:None,product_id:None,class_code:Some(0x0c),subclass:Some(0x03),programming_interface:None};16],match_count:1,restart_limit:3,crashes:0};bus.drivers.register(hid)?;
 let mut pci_functions=0;let mut ids=[DeviceId(0);256];let mut id_count=0;
 pci::enumerate(|f|{pci_functions+=1;if id_count<ids.len(){let mut resources=[Resource::None;8];pci::read_resources(f,&mut resources);let d=Device{id:DeviceId(0),parent:None,bus:BusType::Pci,vendor_id:f.vendor_id,product_id:f.device_id,class_code:f.class_code,subclass:f.subclass,programming_interface:f.programming_interface,revision:f.revision,location:((f.bus as u64)<<16)|((f.device as u64)<<8)|f.function as u64,generation:0,state:DeviceState::Discovered,driver_id:None,resources};if let Ok(id)=bus.devices.insert(d){let generation=bus.devices.get(id).map(|d|d.generation).unwrap_or(0);bus.hotplug.push(id,generation,HotplugKind::Arrived);ids[id_count]=id;id_count+=1;}}});
 let mut bound=0;for id in &ids[..id_count]{if let Ok(driver_id)=bus.drivers.bind_best(&mut bus.devices,*id){bound+=1;let generation=bus.devices.get(*id).map(|d|d.generation).unwrap_or(0);bus.hotplug.push(*id,generation,HotplugKind::Bound);let restart_limit=bus.drivers.get(driver_id).map(|d|d.restart_limit).unwrap_or(0);let _=kernel_runtime::with_runtime(|r|r.watchdog.register(driver_id,*id,0,500,restart_limit));}}
 bus.initialized=true;serial::println(format_args!("[BUS ] ForgeBus retained {} PCI functions, {} devices, 2 built-in drivers, {} bound",pci_functions,bus.devices.count(),bound));Ok(ForgeBusReport{pci_functions,devices:bus.devices.count(),drivers:2,bound})
})}

pub fn initialized()->bool{with_bus(|b|b.initialized)}
pub fn dispatch_interrupt(vector:u8)->Result<DeviceId,&'static str>{
 let (device,handler)=kernel_runtime::with_runtime(|r|{
  let device=r.interrupts.record_dispatch(vector)?;
  Ok::<_,&'static str>((device,r.interrupts.handler(vector,device)))
 })?;
 let handler=handler.ok_or("device interrupt handler disappeared")?;
 handler(vector,device)?;
 Ok(device)
}

/// Executes watchdog decisions instead of merely returning them. Device DMA is
/// fenced before requests or process memory may be released.
pub fn service_watchdog(now:u64,allocator:&mut FrameAllocator<'_>){
 let mut actions=[(0u64,WatchdogAction::None);128];let mut count=0;
 with_bus(|bus|{for dev in bus.devices.iter(){if let Some(driver_id)=dev.driver_id{if count<actions.len(){let action=kernel_runtime::with_runtime(|r|r.watchdog.evaluate(driver_id,dev.id,now).unwrap_or(WatchdogAction::None));actions[count]=(driver_id,action);count+=1;}}}});
 for (driver_id,action) in &actions[..count]{match action{WatchdogAction::None=>{},WatchdogAction::Ping=>serial::println(format_args!("[WDOG] ping driver {}",driver_id)),WatchdogAction::Restart|WatchdogAction::Quarantine=>recover_driver(*driver_id,*action==WatchdogAction::Quarantine,allocator),}}
}

fn recover_driver(driver_id:u64,quarantine:bool,allocator:&mut FrameAllocator<'_>){with_bus(|bus|{
 let mut affected=[DeviceId(0);256];let mut count=0;for d in bus.devices.iter(){if d.driver_id==Some(driver_id)&&count<affected.len(){affected[count]=d.id;count+=1}}
 for device in &affected[..count]{
  // Interrupts are masked first; controller-specific reset follows in its backend.
  kernel_runtime::with_runtime(|r|{for vector in 0x50u8..=0xdf{if r.interrupts.owner(vector)==Some(*device){let _=r.interrupts.mask(vector,*device);}}});
  // ForgeBus only frees DMA after the device is logically detached. Hardware
  // backends must disable bus mastering before calling this path.
  let _=bus.dma.force_revoke(allocator,*device);
  kernel_runtime::with_runtime(|r|{r.block_requests.fence_device(device.0,-5);});
  if let Some(d)=bus.devices.get(*device){bus.hotplug.push(*device,d.generation,if quarantine{HotplugKind::Failed}else{HotplugKind::Authorized});}
 }
 bus.drivers.unbind(&mut bus.devices,driver_id,quarantine);
 if !quarantine{for device in &affected[..count]{let _=bus.drivers.bind_best(&mut bus.devices,*device);}}
 serial::println(format_args!("[WDOG] driver {} {}",driver_id,if quarantine{"quarantined"}else{"restarted"}));
})}

pub fn fence_owner_io(owner:u64,allocator:&mut FrameAllocator<'_>)->bool{
 kernel_runtime::with_runtime(|r|{r.block_requests.request_cancel_owner(owner);});
 let mut devices=[0u64;128];let mut count=0;kernel_runtime::with_runtime(|r|r.block_requests.devices_needing_fence(owner,|d|{if count<devices.len(){devices[count]=d;count+=1}}));
 for id in &devices[..count]{with_bus(|bus|{let device=DeviceId(*id);let _=bus.dma.force_revoke(allocator,device);});kernel_runtime::with_runtime(|r|{r.block_requests.fence_device(*id,-125);});}
 !kernel_runtime::with_runtime(|r|r.block_requests.owner_has_unfenced(owner))
}

/// K13 claims a PCI function through ForgeBus before touching device MMIO or
/// enabling bus mastering. The exact vendor/product match prevents a generic
/// display driver from stealing the adapter during bring-up.
pub fn claim_pci_function(
 function:pci::PciFunction,
 driver_name:&[u8],
 restart_limit:u8,
)->Result<(DeviceId,u64),&'static str>{with_bus(|bus|{
 if !bus.initialized{return Err("ForgeBus is not initialized")}
 let location=((function.bus as u64)<<16)|((function.device as u64)<<8)|function.function as u64;
 let device_id=bus.devices.iter().find(|d|d.location==location&&d.vendor_id==function.vendor_id&&d.product_id==function.device_id).map(|d|d.id).ok_or("PCI function is absent from ForgeBus")?;
 if let Some(existing)=bus.devices.get(device_id).and_then(|d|d.driver_id){return Ok((device_id,existing))}
 let descriptor=DriverDescriptor{
  id:0,
  name:name(driver_name),
  version:1,
  signer:TrustLevel::System,
  isolation:DriverIsolation::Kernel,
  state:DriverState::Registered,
  matches:[DriverMatch{vendor_id:Some(function.vendor_id),product_id:Some(function.device_id),class_code:None,subclass:None,programming_interface:None};16],
  match_count:1,
  restart_limit,
  crashes:0,
 };
 let driver_id=bus.drivers.register(descriptor)?;
 let bound=bus.drivers.bind_best(&mut bus.devices,device_id)?;
 if bound!=driver_id{return Err("ForgeBus bound unexpected driver")}
 let generation=bus.devices.get(device_id).map(|d|d.generation).unwrap_or(0);
 bus.hotplug.push(device_id,generation,HotplugKind::Bound);
 Ok((device_id,driver_id))
})}

/// Establish the bounded software DMA ownership domain used by K13.B. Hardware
/// IOMMU translation is a separate backend responsibility; this API guarantees
/// that all transport pages are tracked and can be revoked as one device unit.
pub fn establish_dma_domain(device:DeviceId,address_bits:u8,coherent:bool)->Result<(),&'static str>{with_bus(|bus|{
 if bus.dma.domain_for(device).is_some(){return Ok(())}
 bus.dma.create_domain(device,address_bits,coherent).map(|_|())
})}

pub fn allocate_dma(
 allocator:&mut FrameAllocator<'_>,
 device:DeviceId,
 bytes:u64,
 direction:DmaDirection,
)->Result<DmaMapping,&'static str>{with_bus(|bus|{
 let domain=bus.dma.domain_for(device).ok_or("ForgeBus DMA domain is unavailable")?;
 domain.map_contiguous(allocator,bytes,direction)
})}

pub fn release_dma(
 allocator:&mut FrameAllocator<'_>,
 device:DeviceId,
 physical:u64,
)->Result<(),&'static str>{with_bus(|bus|{
 let domain=bus.dma.domain_for(device).ok_or("ForgeBus DMA domain is unavailable")?;
 domain.unmap(allocator,physical)
})}

pub fn mark_device_online(device:DeviceId)->Result<(),&'static str>{with_bus(|bus|{
 let entry=bus.devices.get_mut(device).ok_or("ForgeBus device disappeared")?;
 entry.state=DeviceState::Online;
 Ok(())
})}

pub fn revoke_device_dma(allocator:&mut FrameAllocator<'_>,device:DeviceId)->Result<usize,&'static str>{
 with_bus(|bus|bus.dma.force_revoke(allocator,device))
}

/// Resolve the ForgeBus device object that owns an exact PCI requester.
///
/// K14.C27 uses this instead of inventing a second Radeon ownership table.  The
/// lookup is exact on BDF + vendor + product and therefore returns the same
/// retained DeviceId that was created during ForgeBus enumeration.
pub fn device_id_for_pci(function:pci::PciFunction)->Option<DeviceId>{with_bus(|bus|{
 let location=((function.bus as u64)<<16)|((function.device as u64)<<8)|function.function as u64;
 bus.devices.iter().find(|d|d.location==location&&d.vendor_id==function.vendor_id&&d.product_id==function.device_id).map(|d|d.id)
})}

/// Return the currently bound ForgeBus driver for a retained device.
pub fn driver_id_for_device(device:DeviceId)->Option<u64>{with_bus(|bus|{
 bus.devices.get(device).and_then(|d|d.driver_id)
})}

/// Snapshot the retained ForgeBus lifecycle state for a device.
pub fn device_state(device:DeviceId)->Option<DeviceState>{with_bus(|bus|{
 bus.devices.get(device).map(|d|d.state)
})}
