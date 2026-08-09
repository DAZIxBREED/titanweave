//! ACPI MCFG parser and PCIe ECAM access windows.
use crate::{acpi::{self,AcpiCatalog},pci_address::PciAddress};
use core::ptr;
pub const MAX_ECAM_WINDOWS:usize=16;
#[derive(Clone,Copy,Debug)]pub struct EcamWindow{pub base:u64,pub segment:u16,pub start_bus:u8,pub end_bus:u8}
impl EcamWindow{pub const EMPTY:Self=Self{base:0,segment:0,start_bus:0,end_bus:0};pub fn contains(&self,a:PciAddress)->bool{self.base!=0&&self.segment==a.segment&&a.bus>=self.start_bus&&a.bus<=self.end_bus}}
pub struct EcamCatalog{windows:[EcamWindow;MAX_ECAM_WINDOWS],count:usize}
impl EcamCatalog{
 pub const fn empty()->Self{Self{windows:[EcamWindow::EMPTY;MAX_ECAM_WINDOWS],count:0}}
 pub fn from_acpi(c:&AcpiCatalog)->Result<Self,&'static str>{let t=c.find(*b"MCFG",0).ok_or("MCFG not found")?;if t.length<44{return Err("MCFG too short")}let mut out=Self::empty();let mut off=44usize;while off+16<=t.length as usize{if out.count>=MAX_ECAM_WINDOWS{return Err("too many ECAM windows")}let base=acpi::read_table_u64(t,off)?;let segment=acpi::read_table_u16(t,off+8)?;let start=acpi::read_table_u8(t,off+10)?;let end=acpi::read_table_u8(t,off+11)?;if base==0||start>end{return Err("invalid ECAM window")}out.windows[out.count]=EcamWindow{base,segment,start_bus:start,end_bus:end};out.count+=1;off+=16}Ok(out)}
 pub fn window(&self,a:PciAddress)->Option<EcamWindow>{self.windows[..self.count].iter().find(|w|w.contains(a)).copied()}
 pub fn read_u32(&self,a:PciAddress,offset:u16)->Result<u32,&'static str>{if offset>4092||offset&3!=0{return Err("invalid ECAM offset")}let w=self.window(a).ok_or("no ECAM window")?;let address=w.base+(((a.bus-w.start_bus)as u64)<<20)+((a.device as u64)<<15)+((a.function as u64)<<12)+offset as u64;Ok(unsafe{ptr::read_volatile(address as *const u32)})}
 pub fn write_u32(&self,a:PciAddress,offset:u16,value:u32)->Result<(),&'static str>{if offset>4092||offset&3!=0{return Err("invalid ECAM offset")}let w=self.window(a).ok_or("no ECAM window")?;let address=w.base+(((a.bus-w.start_bus)as u64)<<20)+((a.device as u64)<<15)+((a.function as u64)<<12)+offset as u64;unsafe{ptr::write_volatile(address as *mut u32,value)};Ok(())}
}
