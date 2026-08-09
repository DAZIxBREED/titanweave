//! Operational C29 timeline fence storage and SDMA fence packet generation.

use core::ptr;
use crate::{memory::{FrameAllocator,FRAME_SIZE},radeon_memory::{self,RadeonMemoryObject},radeon_sdma_packets,sync::SpinLock};

pub const RADEON_FENCE_ABI_VERSION:u32=1;
pub const C29_FENCE_BYTES:u64=FRAME_SIZE;
#[derive(Clone,Copy,Debug)]
pub struct RadeonFenceState{pub ready:bool,pub object_id:u64,pub gpu_address:u64,pub issued:u32,pub completed:u32,pub packet_verified:bool,pub fingerprint:u64}
impl RadeonFenceState{pub const EMPTY:Self=Self{ready:false,object_id:0,gpu_address:0,issued:0,completed:0,packet_verified:false,fingerprint:0};}
static STATE:SpinLock<RadeonFenceState>=SpinLock::new(RadeonFenceState::EMPTY);

pub struct RadeonFenceTimeline{owner:u64,object:RadeonMemoryObject,issued:u32}
impl RadeonFenceTimeline{
 pub fn allocate(allocator:&mut FrameAllocator<'_>,owner:u64)->Result<Self,&'static str>{let o=radeon_memory::allocate_gtt(allocator,owner,C29_FENCE_BYTES,FRAME_SIZE)?;radeon_memory::pin(owner,o.id,true)?;unsafe{ptr::write_volatile(o.kernel_virtual as *mut u32,0)};Ok(Self{owner,object:o,issued:0})}
 pub fn issue(&mut self)->Result<(u32,radeon_sdma_packets::FencePacket),&'static str>{self.issued=self.issued.checked_add(1).ok_or("C29 fence timeline exhausted")?;let p=radeon_sdma_packets::fence(self.object.gpu_virtual,self.issued)?;Ok((self.issued,p))}
 pub fn complete_software(&self,sequence:u32){unsafe{ptr::write_volatile(self.object.kernel_virtual as *mut u32,sequence)}}
 pub fn completed(&self)->u32{unsafe{ptr::read_volatile(self.object.kernel_virtual as *const u32)}}
 pub fn is_complete(&self,sequence:u32)->bool{self.completed()>=sequence}
 pub fn gpu_address(&self)->u64{self.object.gpu_virtual}
 pub fn publish(&self,packet_verified:bool)->RadeonFenceState{let completed=self.completed();let fp=0xc029_4645_4e43_0001u64^self.object.id^self.object.gpu_virtual^u64::from(self.issued)^((completed as u64)<<32);let s=RadeonFenceState{ready:true,object_id:self.object.id,gpu_address:self.object.gpu_virtual,issued:self.issued,completed,packet_verified,fingerprint:fp};*STATE.lock()=s;s}
 pub fn release(self,allocator:&mut FrameAllocator<'_>)->Result<(),&'static str>{radeon_memory::pin(self.owner,self.object.id,false)?;radeon_memory::free(allocator,self.owner,self.object.id)}
}
pub fn state()->RadeonFenceState{*STATE.lock()}
