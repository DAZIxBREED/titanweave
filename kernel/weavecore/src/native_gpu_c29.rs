//! K14.C29 large integrated Radeon rings + queues + fences + DMA milestone.
//!
//! C29 consumes frozen C28 memory ownership and constructs an operational SDMA
//! execution control plane: a real GTT-backed ring, FIFO submission queue,
//! GTT-backed timeline fence, source-backed SDMA packet codecs, and a bounded
//! typed copy/fence executor over real owned mappings.  On physical GFX12 the
//! exact SDMA0 queue-0 register plan is resolved from the checksum-verified AMD
//! discovery snapshot.  Silicon execution remains fail-closed until its actual
//! firmware, GPU address-translation and persistent IOMMU prerequisites exist;
//! C29 never turns that absence into a fake hardware PASS.

use crate::{
 memory::{FrameAllocator,FRAME_SIZE},native_gpu_c28,pci,radeon_dma,radeon_fence::{self,RadeonFenceTimeline},
 radeon_memory,radeon_queue,radeon_ring::{self,RadeonRing},radeon_sdma,radeon_sdma_packets,serial,sync::SpinLock,
};

pub const K14C29_ABI_VERSION:u32=1;
pub const RADEON_C29_RING_BYTES:u64=16*1024;
pub const RADEON_C29_RAW_PACKET_SUBMISSION:bool=false;
pub const RADEON_C29_CALLER_MMIO_ADDRESS:bool=false;
pub const RADEON_C29_CALLER_MMIO_VALUE:bool=false;
pub const RADEON_C29_UNTRANSLATED_BUS_MASTER:bool=false;
pub const RADEON_C29_PLACEHOLDER_SUBSYSTEMS:u8=0;

#[derive(Clone,Copy,Debug)]
pub struct C29State{
 pub amd_present:bool,pub navi48:bool,pub c28_verified:bool,pub packet_codec_verified:bool,pub ring_ready:bool,pub ring_object:u64,pub ring_gpu_address:u64,pub ring_dwords:u64,
 pub queue_ready:bool,pub queue_order_verified:bool,pub queue_cancel_verified:bool,pub fence_ready:bool,pub fence_object:u64,pub fence_gpu_address:u64,pub fence_sequence:u32,pub fence_completed:u32,
 pub dma_ready:bool,pub typed_submission:bool,pub dma_copy_verified:bool,pub dma_fence_verified:bool,pub dma_bytes:u32,pub sdma_register_plan_verified:bool,pub sdma_gc_base0_resolved:bool,pub sdma_gc_base0_dwords:u64,
 pub firmware_staged:bool,pub firmware_uploaded_to_silicon:bool,pub gpu_address_translation_live:bool,pub persistent_iommu_domain_live:bool,pub bus_master_enabled:bool,pub physical_sdma_programmed:bool,pub hardware_dma_executed:bool,
 pub raw_packets_allowed:bool,pub caller_mmio_allowed:bool,pub hardware_deferred:bool,pub qualified:bool,pub ring_fingerprint:u64,pub queue_fingerprint:u64,pub fence_fingerprint:u64,pub dma_fingerprint:u64,pub sdma_fingerprint:u64,pub qualification_fingerprint:u64,pub fallback_armed:bool,
}
impl C29State{pub const EMPTY:Self=Self{amd_present:false,navi48:false,c28_verified:false,packet_codec_verified:false,ring_ready:false,ring_object:0,ring_gpu_address:0,ring_dwords:0,queue_ready:false,queue_order_verified:false,queue_cancel_verified:false,fence_ready:false,fence_object:0,fence_gpu_address:0,fence_sequence:0,fence_completed:0,dma_ready:false,typed_submission:false,dma_copy_verified:false,dma_fence_verified:false,dma_bytes:0,sdma_register_plan_verified:false,sdma_gc_base0_resolved:false,sdma_gc_base0_dwords:0,firmware_staged:false,firmware_uploaded_to_silicon:false,gpu_address_translation_live:false,persistent_iommu_domain_live:false,bus_master_enabled:false,physical_sdma_programmed:false,hardware_dma_executed:false,raw_packets_allowed:false,caller_mmio_allowed:false,hardware_deferred:true,qualified:false,ring_fingerprint:0,queue_fingerprint:0,fence_fingerprint:0,dma_fingerprint:0,sdma_fingerprint:0,qualification_fingerprint:0,fallback_armed:true};}
static STATE:SpinLock<C29State>=SpinLock::new(C29State::EMPTY);
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}
fn policy_self_test()->Result<(),&'static str>{if K14C29_ABI_VERSION!=1||RADEON_C29_RAW_PACKET_SUBMISSION||RADEON_C29_CALLER_MMIO_ADDRESS||RADEON_C29_CALLER_MMIO_VALUE||RADEON_C29_UNTRANSLATED_BUS_MASTER||RADEON_C29_PLACEHOLDER_SUBSYSTEMS!=0{return Err("K14.C29 policy violates locked roadmap/no-stub contract")}Ok(())}

pub fn initialize(allocator:&mut FrameAllocator<'_>)->Result<C29State,&'static str>{
 policy_self_test()?;let c28=native_gpu_c28::state();if !c28.qualified{return Err("K14.C29 requires frozen qualified C28")}
 let owner=if crate::native_gpu_c27::state().forge_device!=0{crate::native_gpu_c27::state().forge_device}else{0xc029};
 let codec_fp=radeon_sdma_packets::self_test()?;let queue=radeon_queue::self_test()?;
 let mut ring=RadeonRing::allocate(allocator,owner)?;let mut fence=RadeonFenceTimeline::allocate(allocator,owner)?;
 let dma=radeon_dma::qualification(allocator,owner,&mut ring,&mut fence)?;let ring_state=ring.publish(codec_fp);let fence_state=fence.publish(dma.fence_verified);
 let firmware_uploaded=c28.firmware_gpu_upload_performed;let gpu_translation=c28.gpu_page_tables_installed;let iommu_live=false;
 let hardware_prereqs=c28.amd_present&&firmware_uploaded&&gpu_translation&&iommu_live;
 let sdma=radeon_sdma::initialize(hardware_prereqs)?;
 let command=if c28.amd_present{let r=crate::radeon_resources::state();pci::read_u16(r.bus,r.device,r.function,0x04)}else{0};let bus_master=command&(1<<2)!=0;
 if bus_master&&!iommu_live{return Err("C29 detected Radeon bus mastering without persistent translated domain")}
 let mut s=C29State{amd_present:c28.amd_present,navi48:c28.navi48,c28_verified:c28.qualified,packet_codec_verified:codec_fp!=0,ring_ready:ring_state.ready,ring_object:ring_state.object_id,ring_gpu_address:ring_state.gpu_address,ring_dwords:ring_state.capacity_dwords,
  queue_ready:queue.ready,queue_order_verified:queue.ordering_verified,queue_cancel_verified:queue.cancellation_verified,fence_ready:fence_state.ready,fence_object:fence_state.object_id,fence_gpu_address:fence_state.gpu_address,fence_sequence:fence_state.issued,fence_completed:fence_state.completed,
  dma_ready:dma.ready,typed_submission:dma.typed_submission,dma_copy_verified:dma.copy_verified,dma_fence_verified:dma.fence_verified,dma_bytes:dma.bytes_copied,sdma_register_plan_verified:sdma.register_plan_verified,sdma_gc_base0_resolved:sdma.gc_base0_resolved,sdma_gc_base0_dwords:sdma.gc_base0_dwords,
  firmware_staged:c28.firmware_staging_verified,firmware_uploaded_to_silicon:firmware_uploaded,gpu_address_translation_live:gpu_translation,persistent_iommu_domain_live:iommu_live,bus_master_enabled:bus_master,physical_sdma_programmed:sdma.hardware_programmed,hardware_dma_executed:dma.hardware_executed,
  raw_packets_allowed:dma.raw_packets_allowed,caller_mmio_allowed:false,hardware_deferred:!hardware_prereqs,ring_fingerprint:ring_state.fingerprint,queue_fingerprint:queue.fingerprint,fence_fingerprint:fence_state.fingerprint,dma_fingerprint:dma.fingerprint,sdma_fingerprint:sdma.fingerprint,..C29State::EMPTY};
 serial::println(format_args!("[C29RG] SDMA ring: ready={} object={} gpu={:#x} dwords={} emitted={} wraps={} align_dwords={} fingerprint={:#018x}",s.ring_ready,s.ring_object,s.ring_gpu_address,s.ring_dwords,ring_state.emitted_dwords,ring_state.wraps,radeon_ring::C29_RING_ALIGN_DWORDS,s.ring_fingerprint));
 serial::println(format_args!("[C29QU] submission queue: ready={} FIFO={} cancellation={} submitted={} retired={} cancelled={} raw_packets=false fingerprint={:#018x}",s.queue_ready,s.queue_order_verified,s.queue_cancel_verified,queue.submitted,queue.retired,queue.cancelled,s.queue_fingerprint));
 serial::println(format_args!("[C29FN] timeline fence: ready={} object={} gpu={:#x} issued={} completed={} packet_verified={} UC_mtype=3 fingerprint={:#018x}",s.fence_ready,s.fence_object,s.fence_gpu_address,s.fence_sequence,s.fence_completed,fence_state.packet_verified,s.fence_fingerprint));
 serial::println(format_args!("[C29DM] typed SDMA DMA: ready={} submission={} copy={} fence={} bytes={} hardware_executed={} software_executor=true fingerprint={:#018x}",s.dma_ready,s.typed_submission,s.dma_copy_verified,s.dma_fence_verified,s.dma_bytes,s.hardware_dma_executed,s.dma_fingerprint));
 serial::println(format_args!("[C29SD] GFX12 SDMA0 queue0 authority: exact_regs={} GC_base0_resolved={} GC_base0={:#x} RB_CNTL={:#x} RB_BASE={:#x} RB_WPTR={:#x} physical_programmed={} fingerprint={:#018x}",sdma.exact_registers,s.sdma_gc_base0_resolved,s.sdma_gc_base0_dwords,sdma.rb_cntl_byte,sdma.rb_base_byte,sdma.rb_wptr_byte,s.physical_sdma_programmed,s.sdma_fingerprint));
 serial::println(format_args!("[C29PG] execution authority: C28=true typed_packets=true raw_packets=false caller_MMIO=false ring_GTT=true queue=true fence_GTT=true software_DMA=true firmware_staged={} firmware_silicon={} GPU_translation={} persistent_IOMMU={} bus_master={} physical_SDMA={} hardware_DMA={} fail_closed=true",s.firmware_staged,s.firmware_uploaded_to_silicon,s.gpu_address_translation_live,s.persistent_iommu_domain_live,s.bus_master_enabled,s.physical_sdma_programmed,s.hardware_dma_executed));
 let common=s.c28_verified&&s.packet_codec_verified&&s.ring_ready&&s.ring_object!=0&&s.ring_gpu_address!=0&&s.ring_dwords>=4096&&s.queue_ready&&s.queue_order_verified&&s.queue_cancel_verified&&s.fence_ready&&s.fence_sequence>0&&s.fence_completed==s.fence_sequence&&s.dma_ready&&s.typed_submission&&s.dma_copy_verified&&s.dma_fence_verified&&s.dma_bytes==radeon_dma::C29_DMA_TEST_BYTES&&s.sdma_register_plan_verified&&!s.raw_packets_allowed&&!s.caller_mmio_allowed;
 if !common{return Err("K14.C29 operational ring/queue/fence/DMA gates did not close")}
 if s.amd_present{if !s.sdma_gc_base0_resolved{return Err("K14.C29 physical GFX12 missing verified SDMA GC base0")};if !s.hardware_deferred&&!(s.firmware_uploaded_to_silicon&&s.gpu_address_translation_live&&s.persistent_iommu_domain_live&&s.bus_master_enabled&&s.physical_sdma_programmed&&s.hardware_dma_executed){return Err("K14.C29 physical DMA activation incomplete")}}else if !s.hardware_deferred{return Err("K14.C29 QEMU must retain physical Radeon DMA defer")}
 let mut fp=0xc029_5155_414c_0001u64;for v in [s.ring_fingerprint,s.queue_fingerprint,s.fence_fingerprint,s.dma_fingerprint,s.sdma_fingerprint,u64::from(s.dma_bytes),s.ring_gpu_address,s.fence_gpu_address]{fp=mix(fp,v)}s.qualification_fingerprint=fp;s.qualified=fp!=0;
 serial::println(format_args!("[C29RD] K14.C29 rings+queues+fences+DMA ready: amd_present={} navi48={} C28={} ring={} queue={} fence={} DMA={} bytes={} exact_SDMA={} GC_base0={} hardware_deferred={} bus_master={} physical_SDMA={} qualified={} fingerprint={:#018x} fallback=true",s.amd_present,s.navi48,s.c28_verified,s.ring_ready,s.queue_ready,s.fence_ready,s.dma_ready,s.dma_bytes,s.sdma_register_plan_verified,s.sdma_gc_base0_resolved,s.hardware_deferred,s.bus_master_enabled,s.physical_sdma_programmed,s.qualified,s.qualification_fingerprint));
 *STATE.lock()=s;Ok(s)
}
pub fn state()->C29State{*STATE.lock()}
pub fn packed_status()->u64{let s=state();let mut v=(u64::from(s.dma_bytes)<<32);for(bit,on)in[s.amd_present,s.navi48,s.c28_verified,s.packet_codec_verified,s.ring_ready,s.queue_ready,s.queue_order_verified,s.fence_ready,s.dma_ready,s.typed_submission,s.dma_copy_verified,s.dma_fence_verified,s.sdma_register_plan_verified,s.sdma_gc_base0_resolved,s.hardware_deferred,s.qualified,s.fallback_armed].into_iter().enumerate(){if on{v|=1u64<<bit}}v}
