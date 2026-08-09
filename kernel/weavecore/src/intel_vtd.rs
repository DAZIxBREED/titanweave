//! Intel VT-d DMAR discovery and queued invalidation backend.
use crate::{acpi::{self,AcpiCatalog},iommu_core::{Access,PageSpan,TranslationBackend},pci_address::RequesterId};
pub const MAX_DRHD_UNITS:usize=8;pub const VTD_QUEUE_ENTRIES:usize=256;
#[derive(Clone,Copy,Debug)]pub struct DrhdUnit{pub segment:u16,pub register_base:u64,pub include_all:bool,pub enabled:bool}impl DrhdUnit{pub const EMPTY:Self=Self{segment:0,register_base:0,include_all:false,enabled:false};}
#[derive(Clone,Copy)]struct Descriptor{lo:u64,hi:u64}impl Descriptor{const EMPTY:Self=Self{lo:0,hi:0};}
pub struct IntelVtd{pub units:[DrhdUnit;MAX_DRHD_UNITS],pub count:usize,queue:[Descriptor;VTD_QUEUE_ENTRIES],head:u16,tail:u16,blocked:[bool;65536]}
impl IntelVtd{
 pub const fn empty()->Self{Self{units:[DrhdUnit::EMPTY;MAX_DRHD_UNITS],count:0,queue:[Descriptor::EMPTY;VTD_QUEUE_ENTRIES],head:0,tail:0,blocked:[true;65536]}}
 pub fn discover(c:&AcpiCatalog)->Result<Self,&'static str>{let t=c.find(*b"DMAR",0).ok_or("DMAR not found")?;if t.length<48{return Err("DMAR too short")}let mut s=Self::empty();let mut off=48usize;while off+4<=t.length as usize{let ty=acpi::read_table_u16(t,off)?;let len=acpi::read_table_u16(t,off+2)? as usize;if len<4||off+len>t.length as usize{return Err("invalid DMAR structure")}if ty==0{if len<16{return Err("DRHD too short")}if s.count>=MAX_DRHD_UNITS{return Err("too many DRHD units")}let flags=acpi::read_table_u8(t,off+4)?;let segment=acpi::read_table_u16(t,off+6)?;let base=acpi::read_table_u64(t,off+8)?;if base==0||base&0xfff!=0{return Err("invalid VT-d register base")}s.units[s.count]=DrhdUnit{segment,register_base:base,include_all:flags&1!=0,enabled:false};s.count+=1}off+=len}if s.count==0{return Err("DMAR contains no DRHD units")}Ok(s)}
 fn push(&mut self,d:Descriptor)->Result<(),&'static str>{let next=self.tail.wrapping_add(1)%(VTD_QUEUE_ENTRIES as u16);if next==self.head{return Err("VT-d invalidation queue full")}self.queue[self.tail as usize]=d;self.tail=next;Ok(())}
 fn complete(&mut self){self.head=self.tail}
 pub fn enable_units(&mut self)->Result<(),&'static str>{for u in &mut self.units[..self.count]{u.enabled=true}Ok(())}
}
impl TranslationBackend for IntelVtd{
 fn attach(&mut self,r:RequesterId,d:u16)->Result<(),&'static str>{self.push(Descriptor{lo:(r.0 as u64)<<32|d as u64,hi:1})?;self.blocked[r.0 as usize]=false;self.complete();Ok(())}
 fn detach(&mut self,r:RequesterId)->Result<(),&'static str>{self.block_requester(r)}
 fn map_pages(&mut self,d:u16,iova:u64,p:&[PageSpan],a:Access)->Result<(),&'static str>{let pages=p.iter().try_fold(0u64,|x,s|x.checked_add(s.pages as u64).ok_or("VT-d page overflow"))?;if pages==0{return Err("empty VT-d mapping")}self.push(Descriptor{lo:iova|d as u64,hi:pages|((matches!(a,Access::Write|Access::ReadWrite)as u64)<<63)})}
 fn unmap_pages(&mut self,d:u16,iova:u64,pages:u32)->Result<(),&'static str>{self.push(Descriptor{lo:iova|d as u64,hi:(pages as u64)<<32})}
 fn invalidate_domain(&mut self,d:u16)->Result<(),&'static str>{self.push(Descriptor{lo:d as u64,hi:2})?;self.complete();Ok(())}
 fn block_requester(&mut self,r:RequesterId)->Result<(),&'static str>{self.blocked[r.0 as usize]=true;self.push(Descriptor{lo:(r.0 as u64)<<32,hi:0})?;self.complete();Ok(())}
}
