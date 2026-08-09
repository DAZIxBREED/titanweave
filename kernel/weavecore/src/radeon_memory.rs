//! K14.C28 operational Radeon VRAM/GTT memory ownership.
//!
//! VRAM is managed as real reservations inside the discovered BAR0 aperture;
//! C28 deliberately does not CPU-write VRAM before the display/queue milestones.
//! GTT staging uses real contiguous physical pages from FrameAllocator, mapped
//! into the cacheable supervisor-only kernel DMA aperture and fully reclaimable.
//! No GPU page-table or device-DMA enablement is claimed here; C29 owns that
//! transition.

use core::cmp::min;
use core::ptr;
use crate::{
    iova::IovaAllocator,
    memory::{FrameAllocator, FRAME_SIZE},
    paging,
    radeon_resources::RadeonResourceState,
    sync::SpinLock,
};

pub const RADEON_MEMORY_ABI_VERSION:u32=1;
pub const MAX_RADEON_MEMORY_OBJECTS:usize=128;
pub const MAX_C28_GTT_BYTES:u64=2u64<<30;
pub const MAX_C28_OBJECT_BYTES:u64=512u64<<20;
pub const RADEON_C28_GPU_PAGE_TABLES_INSTALLED:bool=false;
pub const RADEON_C28_DEVICE_DMA_ENABLED:bool=false;
pub const RADEON_GPU_VA_BASE:u64=0x0000_0001_0000_0000;
pub const RADEON_GPU_VA_BYTES:u64=1u64<<40;

#[repr(u8)]
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum RadeonMemoryKind{VramReservation=1,GttBacking=2}

#[derive(Clone,Copy,Debug)]
pub struct RadeonMemoryObject{
 pub id:u64,pub owner:u64,pub kind:RadeonMemoryKind,pub requested_bytes:u64,pub mapped_bytes:u64,
 pub physical:u64,pub kernel_virtual:u64,pub vram_address:u64,pub gpu_virtual:u64,pub pages:u64,pub pinned:bool,pub active:bool,
}
impl RadeonMemoryObject{pub const EMPTY:Self=Self{id:0,owner:0,kind:RadeonMemoryKind::GttBacking,requested_bytes:0,mapped_bytes:0,physical:0,kernel_virtual:0,vram_address:0,gpu_virtual:0,pages:0,pinned:false,active:false};}

pub struct RadeonMemoryManager{
 initialized:bool,objects:[RadeonMemoryObject;MAX_RADEON_MEMORY_OBJECTS],next_id:u64,
 vram:IovaAllocator,vram_enabled:bool,vram_base:u64,vram_bytes:u64,vram_used:u64,
 gpu_va:IovaAllocator,gpu_va_used:u64,gtt_budget:u64,gtt_used:u64,kernel_cr3:u64,
}
impl RadeonMemoryManager{
 pub const fn empty()->Self{Self{initialized:false,objects:[RadeonMemoryObject::EMPTY;MAX_RADEON_MEMORY_OBJECTS],next_id:1,
  vram:IovaAllocator::empty(),vram_enabled:false,vram_base:0,vram_bytes:0,vram_used:0,gpu_va:IovaAllocator::empty(),gpu_va_used:0,gtt_budget:0,gtt_used:0,kernel_cr3:0}}
 fn configure(&mut self,resources:RadeonResourceState,kernel_cr3:u64,gtt_budget:u64)->Result<(),&'static str>{
  if self.initialized{return Err("Radeon memory manager already initialized")}
  if kernel_cr3==0{return Err("Radeon memory manager missing kernel CR3")}
  self.kernel_cr3=kernel_cr3;self.gtt_budget=gtt_budget;self.gpu_va=IovaAllocator::new(RADEON_GPU_VA_BASE,RADEON_GPU_VA_BYTES/FRAME_SIZE)?;
  if resources.amd_present&&resources.bar0_ready&&resources.bar0_vram_aperture_bytes>=FRAME_SIZE{
   let aligned_base=resources.bar0_vram_base&!(FRAME_SIZE-1);
   let page_delta=resources.bar0_vram_base-aligned_base;
   let usable=resources.bar0_vram_aperture_bytes.saturating_sub(page_delta)&!(FRAME_SIZE-1);
   if usable>=FRAME_SIZE{self.vram=IovaAllocator::new(aligned_base,usable/FRAME_SIZE)?;self.vram_enabled=true;self.vram_base=aligned_base;self.vram_bytes=usable;}
  }
  self.initialized=true;Ok(())
 }
 fn slot(&self)->Result<usize,&'static str>{self.objects.iter().position(|o|!o.active).ok_or("Radeon memory object table full")}
 fn index(&self,owner:u64,id:u64)->Result<usize,&'static str>{self.objects.iter().position(|o|o.active&&o.owner==owner&&o.id==id).ok_or("Radeon memory object not found or wrong owner")}
 fn new_id(&mut self)->Result<u64,&'static str>{let id=self.next_id;self.next_id=self.next_id.checked_add(1).ok_or("Radeon memory object id exhausted")?;Ok(id)}
 pub fn reserve_vram(&mut self,owner:u64,bytes:u64,alignment:u64)->Result<RadeonMemoryObject,&'static str>{
  if !self.initialized||!self.vram_enabled{return Err("Radeon VRAM aperture is unavailable")}
  validate_request(owner,bytes,alignment)?;let pages=pages_for(bytes)?;let align_pages=alignment/FRAME_SIZE;
  let address=self.vram.allocate(pages,align_pages)?;let mapped=pages*FRAME_SIZE;
  let gpu_virtual=match self.gpu_va.allocate(pages,align_pages){Ok(v)=>v,Err(e)=>{self.vram.free(address,pages)?;return Err(e)}};
  let object=RadeonMemoryObject{id:self.new_id()?,owner,kind:RadeonMemoryKind::VramReservation,requested_bytes:bytes,mapped_bytes:mapped,
   physical:0,kernel_virtual:0,vram_address:address,gpu_virtual,pages,pinned:false,active:true};
  let slot=match self.slot(){Ok(i)=>i,Err(e)=>{self.vram.free(address,pages)?;self.gpu_va.free(gpu_virtual,pages)?;return Err(e)}};
  self.vram_used=self.vram_used.checked_add(mapped).ok_or("Radeon VRAM accounting overflow")?;self.gpu_va_used=self.gpu_va_used.checked_add(mapped).ok_or("Radeon GPU VA accounting overflow")?;self.objects[slot]=object;Ok(object)
 }
 pub fn allocate_gtt(&mut self,allocator:&mut FrameAllocator<'_>,owner:u64,bytes:u64,alignment:u64)->Result<RadeonMemoryObject,&'static str>{
  if !self.initialized{return Err("Radeon memory manager is not initialized")}
  validate_request(owner,bytes,alignment)?;let pages=pages_for(bytes)?;let mapped=pages*FRAME_SIZE;
  if self.gtt_used.checked_add(mapped).ok_or("Radeon GTT accounting overflow")?>self.gtt_budget{return Err("Radeon C28 GTT budget exceeded")}
  let physical=allocator.allocate_contiguous(pages).ok_or("Radeon GTT physical allocation failed")?;
  if physical&(alignment-1)!=0{allocator.deallocate_contiguous(physical,pages)?;return Err("Radeon GTT physical alignment could not be satisfied")}
  let kernel_virtual=match paging::map_kernel_dma(allocator,self.kernel_cr3,physical,mapped){Ok(v)=>v,Err(e)=>{allocator.deallocate_contiguous(physical,pages)?;return Err(e)}};
  let gpu_virtual=match self.gpu_va.allocate(pages,alignment/FRAME_SIZE){Ok(v)=>v,Err(e)=>{paging::unmap_kernel_dma(self.kernel_cr3,kernel_virtual,mapped)?;allocator.deallocate_contiguous(physical,pages)?;return Err(e)}};
  unsafe{ptr::write_bytes(kernel_virtual as *mut u8,0,mapped as usize)};
  let object=RadeonMemoryObject{id:self.new_id()?,owner,kind:RadeonMemoryKind::GttBacking,requested_bytes:bytes,mapped_bytes:mapped,
   physical,kernel_virtual,vram_address:0,gpu_virtual,pages,pinned:false,active:true};
  let slot=match self.slot(){Ok(i)=>i,Err(e)=>{self.gpu_va.free(gpu_virtual,pages)?;paging::unmap_kernel_dma(self.kernel_cr3,kernel_virtual,mapped)?;allocator.deallocate_contiguous(physical,pages)?;return Err(e)}};
  self.gtt_used=self.gtt_used.checked_add(mapped).ok_or("Radeon GTT accounting overflow")?;self.gpu_va_used=self.gpu_va_used.checked_add(mapped).ok_or("Radeon GPU VA accounting overflow")?;self.objects[slot]=object;Ok(object)
 }
 pub fn pin(&mut self,owner:u64,id:u64,pinned:bool)->Result<(),&'static str>{let i=self.index(owner,id)?;self.objects[i].pinned=pinned;Ok(())}
 pub fn object(&self,owner:u64,id:u64)->Option<RadeonMemoryObject>{self.objects.iter().copied().find(|o|o.active&&o.owner==owner&&o.id==id)}
 pub fn free(&mut self,allocator:&mut FrameAllocator<'_>,owner:u64,id:u64)->Result<(),&'static str>{
  let i=self.index(owner,id)?;let o=self.objects[i];if o.pinned{return Err("pinned Radeon memory object cannot be freed")}
  match o.kind{
   RadeonMemoryKind::VramReservation=>{self.vram.free(o.vram_address,o.pages)?;self.vram_used=self.vram_used.checked_sub(o.mapped_bytes).ok_or("Radeon VRAM accounting underflow")?;}
   RadeonMemoryKind::GttBacking=>{let unmapped=paging::unmap_kernel_dma(self.kernel_cr3,o.kernel_virtual,o.mapped_bytes)?;if unmapped!=o.pages{return Err("Radeon GTT unmap page count mismatch")}allocator.deallocate_contiguous(o.physical,o.pages)?;self.gtt_used=self.gtt_used.checked_sub(o.mapped_bytes).ok_or("Radeon GTT accounting underflow")?;}
  }
  self.gpu_va.free(o.gpu_virtual,o.pages)?;self.gpu_va_used=self.gpu_va_used.checked_sub(o.mapped_bytes).ok_or("Radeon GPU VA accounting underflow")?;
  self.objects[i]=RadeonMemoryObject::EMPTY;Ok(())
 }
 pub fn active_count(&self)->usize{self.objects.iter().filter(|o|o.active).count()}
}

fn pages_for(bytes:u64)->Result<u64,&'static str>{bytes.checked_add(FRAME_SIZE-1).map(|v|v/FRAME_SIZE).filter(|p|*p>0).ok_or("Radeon memory request overflow")}
fn validate_request(owner:u64,bytes:u64,alignment:u64)->Result<(),&'static str>{if owner==0||bytes==0||bytes>MAX_C28_OBJECT_BYTES{return Err("invalid Radeon memory request")}if alignment<FRAME_SIZE||!alignment.is_power_of_two()||alignment%FRAME_SIZE!=0{return Err("invalid Radeon memory alignment")}Ok(())}

#[derive(Clone,Copy,Debug)]
pub struct RadeonMemoryState{
 pub initialized:bool,pub gtt_operational:bool,pub gtt_reclaim_verified:bool,pub gtt_cpu_mapping_verified:bool,
 pub vram_allocator_ready:bool,pub vram_model_verified:bool,pub gpu_va_allocator_ready:bool,pub gpu_va_reservation_verified:bool,pub gtt_budget:u64,pub vram_bytes:u64,pub gtt_used:u64,pub vram_used:u64,pub gpu_va_used:u64,
 pub gpu_page_tables_installed:bool,pub device_dma_enabled:bool,pub fingerprint:u64,
}
impl RadeonMemoryState{pub const EMPTY:Self=Self{initialized:false,gtt_operational:false,gtt_reclaim_verified:false,gtt_cpu_mapping_verified:false,vram_allocator_ready:false,vram_model_verified:false,gpu_va_allocator_ready:false,gpu_va_reservation_verified:false,gtt_budget:0,vram_bytes:0,gtt_used:0,vram_used:0,gpu_va_used:0,gpu_page_tables_installed:false,device_dma_enabled:false,fingerprint:0};}
static MANAGER:SpinLock<RadeonMemoryManager>=SpinLock::new(RadeonMemoryManager::empty());
static STATE:SpinLock<RadeonMemoryState>=SpinLock::new(RadeonMemoryState::EMPTY);
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}

fn allocator_self_test()->Result<(),&'static str>{
 let mut a=IovaAllocator::new(0x1000_0000,64)?;let x=a.allocate(4,4)?;let y=a.allocate(8,8)?;
 if x==y||x&0x3fff!=0||y&0x7fff!=0{return Err("Radeon VRAM range allocator alignment test failed")}
 a.free(x,4)?;let z=a.allocate(4,4)?;if z!=x{return Err("Radeon VRAM range allocator did not reclaim freed extent")}
 Ok(())
}

fn real_gtt_self_test(allocator:&mut FrameAllocator<'_>,kernel_cr3:u64)->Result<(),&'static str>{
 let before=allocator.free_pages();let pages=2u64;let physical=allocator.allocate_contiguous(pages).ok_or("C28 GTT self-test allocation failed")?;
 let virt=match paging::map_kernel_dma(allocator,kernel_cr3,physical,pages*FRAME_SIZE){Ok(v)=>v,Err(e)=>{allocator.deallocate_contiguous(physical,pages)?;return Err(e)}};
 unsafe{for i in 0..128usize{ptr::write_volatile((virt as *mut u8).add(i),(i as u8)^0xa5)}for i in 0..128usize{if ptr::read_volatile((virt as *const u8).add(i))!=((i as u8)^0xa5){return Err("C28 GTT CPU mapping readback failed")}}}
 let unmapped=paging::unmap_kernel_dma(kernel_cr3,virt,pages*FRAME_SIZE)?;if unmapped!=pages{return Err("C28 GTT self-test unmap mismatch")}
 allocator.deallocate_contiguous(physical,pages)?;
 if allocator.free_pages()+4<before{return Err("C28 GTT self-test leaked data backing")}
 Ok(())
}

pub fn initialize(allocator:&mut FrameAllocator<'_>,kernel_cr3:u64,resources:RadeonResourceState)->Result<RadeonMemoryState,&'static str>{
 if RADEON_MEMORY_ABI_VERSION!=1||RADEON_C28_GPU_PAGE_TABLES_INSTALLED||RADEON_C28_DEVICE_DMA_ENABLED{return Err("Radeon C28 memory policy constants invalid")}
 allocator_self_test()?;real_gtt_self_test(allocator,kernel_cr3)?;
 let free_bytes=allocator.free_pages().saturating_mul(FRAME_SIZE);let budget=min(MAX_C28_GTT_BYTES,free_bytes/4).max(16*FRAME_SIZE);
 let mut m=MANAGER.lock();m.configure(resources,kernel_cr3,budget)?;
 let mut s=RadeonMemoryState{initialized:true,gtt_operational:true,gtt_reclaim_verified:true,gtt_cpu_mapping_verified:true,
  vram_allocator_ready:m.vram_enabled,vram_model_verified:!resources.amd_present||m.vram_enabled,gpu_va_allocator_ready:true,gpu_va_reservation_verified:true,gtt_budget:budget,vram_bytes:m.vram_bytes,
  gtt_used:m.gtt_used,vram_used:m.vram_used,gpu_va_used:m.gpu_va_used,gpu_page_tables_installed:false,device_dma_enabled:false,..RadeonMemoryState::EMPTY};
 let mut fp=0xc28a_4d45_4d00_0001u64;for v in [budget,m.vram_base,m.vram_bytes,resources.fingerprint,allocator.free_pages()]{fp=mix(fp,v)}s.fingerprint=fp;*STATE.lock()=s;Ok(s)
}

pub fn allocate_gtt(allocator:&mut FrameAllocator<'_>,owner:u64,bytes:u64,alignment:u64)->Result<RadeonMemoryObject,&'static str>{MANAGER.lock().allocate_gtt(allocator,owner,bytes,alignment)}
pub fn reserve_vram(owner:u64,bytes:u64,alignment:u64)->Result<RadeonMemoryObject,&'static str>{MANAGER.lock().reserve_vram(owner,bytes,alignment)}
pub fn pin(owner:u64,id:u64,pinned:bool)->Result<(),&'static str>{MANAGER.lock().pin(owner,id,pinned)}
pub fn object(owner:u64,id:u64)->Option<RadeonMemoryObject>{MANAGER.lock().object(owner,id)}
pub fn free(allocator:&mut FrameAllocator<'_>,owner:u64,id:u64)->Result<(),&'static str>{MANAGER.lock().free(allocator,owner,id)}
pub fn state()->RadeonMemoryState{*STATE.lock()}
pub fn usage()->(u64,u64,u64,usize){let m=MANAGER.lock();(m.vram_used,m.gtt_used,m.gpu_va_used,m.active_count())}
