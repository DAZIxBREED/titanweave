//! Source-reviewed GFX12 SDMA0 queue-0 register authority for K14.C29.
//!
//! C29 materializes the exact register plan and verified GC base-0 resolution.
//! Physical writes are not performed until firmware-in-silicon, GPU address
//! translation and the persistent translated DMA domain are all proven live.

use crate::{native_gpu_c19,native_gpu_c21,sync::SpinLock};
pub const RADEON_SDMA_ABI_VERSION:u32=1;
pub const SDMA0_QUEUE0_RB_CNTL:u32=0x0080;
pub const SDMA0_QUEUE0_RB_BASE:u32=0x0081;
pub const SDMA0_QUEUE0_RB_BASE_HI:u32=0x0082;
pub const SDMA0_QUEUE0_RB_RPTR:u32=0x0083;
pub const SDMA0_QUEUE0_RB_RPTR_HI:u32=0x0084;
pub const SDMA0_QUEUE0_RB_WPTR:u32=0x0085;
pub const SDMA0_QUEUE0_RB_WPTR_HI:u32=0x0086;
pub const SDMA0_QUEUE0_RB_RPTR_ADDR_LO:u32=0x0087;
pub const SDMA0_QUEUE0_RB_RPTR_ADDR_HI:u32=0x0088;
pub const SDMA0_QUEUE0_IB_CNTL:u32=0x0089;
pub const SDMA0_QUEUE0_DOORBELL:u32=0x008f;
pub const SDMA0_QUEUE0_DOORBELL_OFFSET:u32=0x0091;
pub const SDMA_QUEUE_BASE_IDX:u8=0;
pub const SDMA_RB_ENABLE_MASK:u32=1;
pub const SDMA_RB_SIZE_MASK:u32=0x3e;
pub const SDMA_RB_PRIV_SHIFT:u32=23;
pub const C29_ARBITRARY_MMIO_ALLOWED:bool=false;
pub const C29_CALLER_REGISTER_ALLOWED:bool=false;
pub const C29_CALLER_VALUE_ALLOWED:bool=false;

#[derive(Clone,Copy,Debug)]
pub struct RadeonSdmaState{pub exact_registers:bool,pub gc_base0_resolved:bool,pub gc_base0_dwords:u64,pub rb_cntl_byte:u64,pub rb_base_byte:u64,pub rb_wptr_byte:u64,pub register_plan_verified:bool,pub hardware_prerequisites:bool,pub hardware_programmed:bool,pub fingerprint:u64}
impl RadeonSdmaState{pub const EMPTY:Self=Self{exact_registers:false,gc_base0_resolved:false,gc_base0_dwords:0,rb_cntl_byte:0,rb_base_byte:0,rb_wptr_byte:0,register_plan_verified:false,hardware_prerequisites:false,hardware_programmed:false,fingerprint:0};}
static STATE:SpinLock<RadeonSdmaState>=SpinLock::new(RadeonSdmaState::EMPTY);
fn byte_offset(base:u64,reg:u32)->Result<u64,&'static str>{base.checked_add(reg as u64).and_then(|v|v.checked_mul(4)).ok_or("C29 SDMA register offset overflow")}
pub fn initialize(hardware_prerequisites:bool)->Result<RadeonSdmaState,&'static str>{
 let mut s=RadeonSdmaState{exact_registers:true,hardware_prerequisites,..RadeonSdmaState::EMPTY};
 if let Some(r)=native_gpu_c19::with_verified_snapshot(|b|native_gpu_c21::resolve_gfx12_scratch_reg0(b)){
  let t=r?;if !t.valid||t.gc_segment0_base_dwords==0{return Err("C29 verified GFX12 GC base0 unavailable")};s.gc_base0_resolved=true;s.gc_base0_dwords=t.gc_segment0_base_dwords;
  s.rb_cntl_byte=byte_offset(s.gc_base0_dwords,SDMA0_QUEUE0_RB_CNTL)?;s.rb_base_byte=byte_offset(s.gc_base0_dwords,SDMA0_QUEUE0_RB_BASE)?;s.rb_wptr_byte=byte_offset(s.gc_base0_dwords,SDMA0_QUEUE0_RB_WPTR)?;
 }
 s.register_plan_verified=SDMA0_QUEUE0_RB_CNTL+1==SDMA0_QUEUE0_RB_BASE&&SDMA0_QUEUE0_RB_BASE+4==SDMA0_QUEUE0_RB_WPTR&&SDMA_QUEUE_BASE_IDX==0;
 // Deliberate fail-closed boundary: no physical register write is made unless
 // all execution prerequisites are supplied by later bare-metal qualification.
 s.hardware_programmed=false;
 s.fingerprint=0xc029_5344_4d41_0001u64^s.gc_base0_dwords^s.rb_cntl_byte^s.rb_base_byte^s.rb_wptr_byte^(s.register_plan_verified as u64);
 *STATE.lock()=s;Ok(s)
}
pub fn state()->RadeonSdmaState{*STATE.lock()}
