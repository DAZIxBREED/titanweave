//! K14.C32 native Radeon multi-GPU enumeration groundwork.
//!
//! Enumerates display-class PCI functions without changing ownership or power.
//! It is a real topology inventory, not a claim that peer DMA or synchronized
//! physical execution has been enabled.

use crate::pci;

pub const RADEON_MULTIGPU_ABI_VERSION:u32=1;
pub const MAX_C32_GPU_ADAPTERS:usize=8;
pub const AMD_VENDOR_ID:u16=0x1002;
pub const RADEON_C32_PEER_DMA_ENABLED:bool=false;
pub const RADEON_C32_CROSS_GPU_EXECUTION_ENABLED:bool=false;
#[derive(Clone,Copy,Debug)]pub struct AdapterIdentity{pub present:bool,pub vendor:u16,pub device:u16,pub bus:u8,pub slot:u8,pub function:u8,pub display_class:bool,pub amd:bool}
impl AdapterIdentity{pub const EMPTY:Self=Self{present:false,vendor:0,device:0,bus:0,slot:0,function:0,display_class:false,amd:false};}
#[derive(Clone,Copy,Debug)]pub struct MultiGpuInventory{pub adapters:[AdapterIdentity;MAX_C32_GPU_ADAPTERS],pub display_adapters:u8,pub amd_adapters:u8,pub overflow:u8,pub peer_dma_enabled:bool,pub cross_gpu_execution_enabled:bool,pub fingerprint:u64}
impl MultiGpuInventory{pub const EMPTY:Self=Self{adapters:[AdapterIdentity::EMPTY;MAX_C32_GPU_ADAPTERS],display_adapters:0,amd_adapters:0,overflow:0,peer_dma_enabled:false,cross_gpu_execution_enabled:false,fingerprint:0};}
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}
pub fn enumerate()->Result<MultiGpuInventory,&'static str>{if RADEON_MULTIGPU_ABI_VERSION!=1||MAX_C32_GPU_ADAPTERS!=8||RADEON_C32_PEER_DMA_ENABLED||RADEON_C32_CROSS_GPU_EXECUTION_ENABLED{return Err("C32 multi-GPU policy invalid")}let mut out=MultiGpuInventory::EMPTY;let mut idx=0usize;pci::enumerate(|f|{if f.class_code!=0x03{return}out.display_adapters=out.display_adapters.saturating_add(1);if f.vendor_id==AMD_VENDOR_ID{out.amd_adapters=out.amd_adapters.saturating_add(1)}if idx<MAX_C32_GPU_ADAPTERS{out.adapters[idx]=AdapterIdentity{present:true,vendor:f.vendor_id,device:f.device_id,bus:f.bus,slot:f.device,function:f.function,display_class:true,amd:f.vendor_id==AMD_VENDOR_ID};idx+=1}else{out.overflow=out.overflow.saturating_add(1)}});let mut h=0xc032_4d47_5055_0001u64;for a in out.adapters.iter().filter(|a|a.present){h=mix(h,u64::from(a.vendor)|(u64::from(a.device)<<16)|(u64::from(a.bus)<<32)|(u64::from(a.slot)<<40)|(u64::from(a.function)<<48))}h=mix(h,u64::from(out.display_adapters)|(u64::from(out.amd_adapters)<<8)|(u64::from(out.overflow)<<16));out.fingerprint=h;Ok(out)}
pub fn self_test()->Result<u64,&'static str>{let a=AdapterIdentity{present:true,vendor:AMD_VENDOR_ID,device:0x7550,bus:3,slot:0,function:0,display_class:true,amd:true};if !a.present||!a.display_class||!a.amd||a.vendor!=AMD_VENDOR_ID{return Err("C32 multi-GPU identity self-test failed")}Ok(0xc032_4d47_5354_0001u64^u64::from(a.vendor)^((a.device as u64)<<16))}
