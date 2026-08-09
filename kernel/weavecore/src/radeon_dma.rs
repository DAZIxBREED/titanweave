//! C29 bounded typed SDMA submission and executable software qualification.
//!
//! The software executor consumes the same typed SDMA COPY/FENCE stream that a
//! physical SDMA ring would consume.  It is intentionally limited to memory
//! objects owned by this driver and never accepts caller-selected raw packets.

use core::ptr;
use crate::{memory::{FrameAllocator,FRAME_SIZE},radeon_fence::RadeonFenceTimeline,radeon_memory::{self,RadeonMemoryObject,RadeonMemoryKind},radeon_ring::RadeonRing,radeon_queue::RadeonSubmissionQueue,radeon_sdma_packets,sync::SpinLock};

pub const RADEON_DMA_ABI_VERSION:u32=1;
pub const C29_DMA_TEST_BYTES:u32=4096;
pub const C29_MAX_SUBMISSION_BYTES:u32=radeon_sdma_packets::SDMA_COPY_MAX_BYTES;
#[derive(Clone,Copy,Debug)]
pub struct RadeonDmaState{pub ready:bool,pub typed_submission:bool,pub copy_verified:bool,pub fence_verified:bool,pub queue_order_verified:bool,pub bytes_copied:u32,pub submissions:u32,pub raw_packets_allowed:bool,pub hardware_executed:bool,pub fingerprint:u64}
impl RadeonDmaState{pub const EMPTY:Self=Self{ready:false,typed_submission:false,copy_verified:false,fence_verified:false,queue_order_verified:false,bytes_copied:0,submissions:0,raw_packets_allowed:false,hardware_executed:false,fingerprint:0};}
static STATE:SpinLock<RadeonDmaState>=SpinLock::new(RadeonDmaState::EMPTY);
fn object_bounds(o:RadeonMemoryObject,address:u64,bytes:u32)->Result<u64,&'static str>{if !o.active||o.kind!=RadeonMemoryKind::GttBacking{return Err("C29 DMA requires active GTT object")}if address<o.gpu_virtual{return Err("C29 DMA address before object")}let off=address-o.gpu_virtual;if off.checked_add(bytes as u64).filter(|e|*e<=o.mapped_bytes).is_none(){return Err("C29 DMA range outside object")}Ok(off)}

pub fn execute_typed_copy(source:RadeonMemoryObject,destination:RadeonMemoryObject,packet:&radeon_sdma_packets::CopyLinearPacket)->Result<u32,&'static str>{
 let (src,dst,bytes)=radeon_sdma_packets::decode_copy(&packet.words)?;let so=object_bounds(source,src,bytes)?;let doff=object_bounds(destination,dst,bytes)?;
 unsafe{ptr::copy_nonoverlapping((source.kernel_virtual+so) as *const u8,(destination.kernel_virtual+doff) as *mut u8,bytes as usize)};Ok(bytes)
}
pub fn execute_typed_fence(timeline:&RadeonFenceTimeline,packet:&radeon_sdma_packets::FencePacket)->Result<u32,&'static str>{let (addr,seq)=radeon_sdma_packets::decode_fence(&packet.words)?;if addr!=timeline.gpu_address(){return Err("C29 fence packet targets wrong timeline")}timeline.complete_software(seq);Ok(seq)}

pub fn qualification(allocator:&mut FrameAllocator<'_>,owner:u64,ring:&mut RadeonRing,timeline:&mut RadeonFenceTimeline)->Result<RadeonDmaState,&'static str>{
 let src=radeon_memory::allocate_gtt(allocator,owner,C29_DMA_TEST_BYTES as u64,FRAME_SIZE)?;let dst=radeon_memory::allocate_gtt(allocator,owner,C29_DMA_TEST_BYTES as u64,FRAME_SIZE)?;
 for i in 0..C29_DMA_TEST_BYTES as usize{unsafe{ptr::write_volatile((src.kernel_virtual as *mut u8).add(i),(i as u8).wrapping_mul(29)^0xa5);ptr::write_volatile((dst.kernel_virtual as *mut u8).add(i),0)}}
 let copy=radeon_sdma_packets::copy_linear(src.gpu_virtual,dst.gpu_virtual,C29_DMA_TEST_BYTES)?;
 let (seq,fence)=timeline.issue()?;
 let mut queue=RadeonSubmissionQueue::new();let submission=queue.enqueue(C29_DMA_TEST_BYTES,seq)?;
 ring.emit(&copy.words)?;ring.emit(&fence.words)?;let _pad=ring.pad_commit_alignment()?;queue.mark_emitted(submission)?;
 let copied=execute_typed_copy(src,dst,&copy)?;let completed=execute_typed_fence(timeline,&fence)?;
 let retired=queue.retire_head(completed)?.ok_or("C29 DMA queue did not retire completed submission")?;
 let mut ok=copied==C29_DMA_TEST_BYTES&&completed==seq&&timeline.is_complete(seq)&&retired.id==submission&&retired.fence==seq;
 for i in 0..C29_DMA_TEST_BYTES as usize{let a=unsafe{ptr::read_volatile((src.kernel_virtual as *const u8).add(i))};let b=unsafe{ptr::read_volatile((dst.kernel_virtual as *const u8).add(i))};if a!=b{ok=false;break}}
 ring.consume_to_wptr();radeon_memory::free(allocator,owner,src.id)?;radeon_memory::free(allocator,owner,dst.id)?;
 if !ok{return Err("C29 typed SDMA copy/fence executor verification failed")}
 let fp=0xc029_444d_4100_0001u64^u64::from(copied)^((seq as u64)<<32)^ring.gpu_address()^timeline.gpu_address();
 let s=RadeonDmaState{ready:true,typed_submission:true,copy_verified:true,fence_verified:true,queue_order_verified:true,bytes_copied:copied,submissions:1,raw_packets_allowed:false,hardware_executed:false,fingerprint:fp};*STATE.lock()=s;Ok(s)
}
pub fn state()->RadeonDmaState{*STATE.lock()}
