//! Backend-neutral translated DMA core with default-deny attachment.
use crate::{device::DeviceId,iova::IovaAllocator,pci_address::RequesterId};
pub const MAX_IOMMU_DOMAINS:usize=128;pub const MAX_IOMMU_MAPPINGS:usize=256;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Access{Read,Write,ReadWrite}
#[derive(Clone,Copy,Debug)]pub struct PageSpan{pub physical:u64,pub pages:u32}impl PageSpan{pub const EMPTY:Self=Self{physical:0,pages:0};}
#[derive(Clone,Copy,Debug)]pub struct Translation{pub iova:u64,pub pages:u32,pub access:Access,pub active:bool}impl Translation{pub const EMPTY:Self=Self{iova:0,pages:0,access:Access::ReadWrite,active:false};}
pub trait TranslationBackend{fn attach(&mut self,requester:RequesterId,domain:u16)->Result<(),&'static str>;fn detach(&mut self,requester:RequesterId)->Result<(),&'static str>;fn map_pages(&mut self,domain:u16,iova:u64,pages:&[PageSpan],access:Access)->Result<(),&'static str>;fn unmap_pages(&mut self,domain:u16,iova:u64,pages:u32)->Result<(),&'static str>;fn invalidate_domain(&mut self,domain:u16)->Result<(),&'static str>;fn block_requester(&mut self,requester:RequesterId)->Result<(),&'static str>;}
pub struct Domain{pub id:u16,pub owner:DeviceId,pub requester:RequesterId,pub attached:bool,iova:IovaAllocator,maps:[Translation;MAX_IOMMU_MAPPINGS]}
impl Domain{
 pub fn new(id:u16,owner:DeviceId,requester:RequesterId)->Result<Self,&'static str>{Ok(Self{id,owner,requester,attached:false,iova:IovaAllocator::new(0x1_0000,0x0fff_0000/4096)?,maps:[Translation::EMPTY;MAX_IOMMU_MAPPINGS]})}
 pub fn attach(&mut self,b:&mut impl TranslationBackend)->Result<(),&'static str>{b.block_requester(self.requester)?;b.attach(self.requester,self.id)?;self.attached=true;Ok(())}
 pub fn map(&mut self,b:&mut impl TranslationBackend,spans:&[PageSpan],access:Access)->Result<u64,&'static str>{if !self.attached||spans.is_empty(){return Err("IOMMU domain unavailable")}let pages=spans.iter().try_fold(0u64,|a,s|if s.pages==0||s.physical&4095!=0{Err("invalid pinned span")}else{a.checked_add(s.pages as u64).ok_or("IOMMU page overflow")})?;let iova=self.iova.allocate(pages,1)?;if let Err(e)=b.map_pages(self.id,iova,spans,access){let _=self.iova.free(iova,pages);return Err(e)}b.invalidate_domain(self.id)?;let m=self.maps.iter_mut().find(|m|!m.active).ok_or("IOMMU map table full")?;*m=Translation{iova,pages:pages as u32,access,active:true};Ok(iova)}
 pub fn unmap(&mut self,b:&mut impl TranslationBackend,iova:u64)->Result<(),&'static str>{let m=self.maps.iter_mut().find(|m|m.active&&m.iova==iova).ok_or("IOMMU mapping not found")?;b.unmap_pages(self.id,m.iova,m.pages)?;b.invalidate_domain(self.id)?;self.iova.free(m.iova,m.pages as u64)?;m.active=false;Ok(())}
 pub fn fence(&mut self,b:&mut impl TranslationBackend)->Result<FenceToken,&'static str>{b.block_requester(self.requester)?;b.detach(self.requester)?;b.invalidate_domain(self.id)?;self.attached=false;Ok(FenceToken{device:self.owner,generation:self.id as u64})}
}
#[derive(Clone,Copy,Debug)]pub struct FenceToken{pub device:DeviceId,pub generation:u64}
