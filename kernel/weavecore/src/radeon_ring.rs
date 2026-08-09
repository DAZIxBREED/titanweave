//! Operational C29 SDMA ring backed by reclaimable C28 GTT memory.

use core::ptr;
use crate::{memory::{FrameAllocator,FRAME_SIZE},radeon_memory::{self,RadeonMemoryObject},radeon_sdma_packets,sync::SpinLock};

pub const RADEON_RING_ABI_VERSION:u32=1;
pub const C29_RING_BYTES:u64=16*1024;
pub const C29_RING_DWORDS:u64=C29_RING_BYTES/4;
pub const C29_RING_ALIGN_DWORDS:u64=16;

#[derive(Clone,Copy,Debug)]
pub struct RadeonRingState{pub ready:bool,pub object_id:u64,pub gpu_address:u64,pub kernel_address:u64,pub capacity_dwords:u64,pub rptr:u64,pub wptr:u64,pub emitted_dwords:u64,pub wraps:u64,pub fingerprint:u64}
impl RadeonRingState{pub const EMPTY:Self=Self{ready:false,object_id:0,gpu_address:0,kernel_address:0,capacity_dwords:0,rptr:0,wptr:0,emitted_dwords:0,wraps:0,fingerprint:0};}
static STATE:SpinLock<RadeonRingState>=SpinLock::new(RadeonRingState::EMPTY);

pub struct RadeonRing{owner:u64,object:RadeonMemoryObject,rptr:u64,wptr:u64,emitted:u64,wraps:u64}
impl RadeonRing{
 pub fn allocate(allocator:&mut FrameAllocator<'_>,owner:u64)->Result<Self,&'static str>{
  let object=radeon_memory::allocate_gtt(allocator,owner,C29_RING_BYTES,FRAME_SIZE)?;
  if object.kernel_virtual==0||object.gpu_virtual==0||object.mapped_bytes<C29_RING_BYTES{return Err("C29 ring GTT backing is invalid")}
  radeon_memory::pin(owner,object.id,true)?;
  Ok(Self{owner,object,rptr:0,wptr:0,emitted:0,wraps:0})
 }
 pub fn gpu_address(&self)->u64{self.object.gpu_virtual}
 pub fn kernel_address(&self)->u64{self.object.kernel_virtual}
 pub fn object_id(&self)->u64{self.object.id}
 pub fn capacity_dwords(&self)->u64{C29_RING_DWORDS}
 pub fn write(&mut self,word:u32)->Result<(),&'static str>{
  let next=(self.wptr+1)%C29_RING_DWORDS;if next==self.rptr{return Err("C29 SDMA ring full")}
  unsafe{ptr::write_volatile((self.object.kernel_virtual as *mut u32).add(self.wptr as usize),word)};
  self.wptr=next;self.emitted=self.emitted.checked_add(1).ok_or("C29 ring emitted counter overflow")?;if self.wptr==0{self.wraps+=1}Ok(())
 }
 pub fn emit(&mut self,words:&[u32])->Result<(),&'static str>{for &w in words{self.write(w)?}Ok(())}
 pub fn pad_commit_alignment(&mut self)->Result<u64,&'static str>{let mut n=0;while self.wptr&(C29_RING_ALIGN_DWORDS-1)!=0{self.write(radeon_sdma_packets::nop())?;n+=1}Ok(n)}
 pub fn read_at(&self,index:u64)->Result<u32,&'static str>{if index>=C29_RING_DWORDS{return Err("C29 ring read outside backing")}Ok(unsafe{ptr::read_volatile((self.object.kernel_virtual as *const u32).add(index as usize))})}
 pub fn consume_to_wptr(&mut self){self.rptr=self.wptr}
 pub fn publish(&self,codec_fp:u64)->RadeonRingState{let fp=0xc029_5249_4e47_0001u64^self.object.id^self.object.gpu_virtual^self.emitted^self.wraps^codec_fp;let s=RadeonRingState{ready:true,object_id:self.object.id,gpu_address:self.object.gpu_virtual,kernel_address:self.object.kernel_virtual,capacity_dwords:C29_RING_DWORDS,rptr:self.rptr,wptr:self.wptr,emitted_dwords:self.emitted,wraps:self.wraps,fingerprint:fp};*STATE.lock()=s;s}
 pub fn release(mut self,allocator:&mut FrameAllocator<'_>)->Result<(),&'static str>{radeon_memory::pin(self.owner,self.object.id,false)?;self.consume_to_wptr();radeon_memory::free(allocator,self.owner,self.object.id)}
}
pub fn state()->RadeonRingState{*STATE.lock()}
