//! Deterministic K11 fault injection and lifecycle stress tests.
use crate::{device::DeviceId,iova::IovaAllocator,pcie_hotplug::HotplugController,pci_address::PciAddress,usb_hid_full::{HidDevice,HidProtocol},xhci::{Trb,TrbRing}};
#[derive(Clone,Copy,Debug)]pub struct StressReport{pub passed:u32,pub failed:u32}
fn check(ok:bool,r:&mut StressReport){if ok{r.passed+=1}else{r.failed+=1}}
pub fn run()->StressReport{let mut r=StressReport{passed:0,failed:0};
 let mut i=IovaAllocator::new(0x10000,128).unwrap();let a=i.allocate(8,1).unwrap();let b=i.allocate(8,8).unwrap();check(a!=b,&mut r);check(i.free(a,8).is_ok(),&mut r);check(i.free(b,8).is_ok(),&mut r);
 let mut ring=TrbRing::new();for _ in 0..255{let _=ring.push(Trb::EMPTY);}check(ring.push(Trb::EMPTY).is_err(),&mut r);
 let mut hp=HotplugController::new();let pa=PciAddress{segment:0,bus:1,device:0,function:0};check(hp.register_slot(pa,1).is_ok(),&mut r);let _=hp.presence_change(pa,1,true,10);let _=hp.poll(25,|_,_,_|{},|_,_|{});check(hp.bind_device(pa,1,DeviceId(7),1).is_err(),&mut r);
 let mut hid=HidDevice::new(1,0,HidProtocol::BootMouse,4).unwrap();hid.start();check(hid.decode_mouse(&[1,2,3,4]).is_ok(),&mut r);check(hid.decode_mouse(&[1,2]).is_err(),&mut r);
 r}
