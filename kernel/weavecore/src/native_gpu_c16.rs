//! K14.C16 reviewed Radeon MMIO-write target and transaction gate.
//!
//! C16 is the first milestone that contains a Radeon MMIO store executor, but
//! it remains fail-closed unless a source-reviewed register target has an exact
//! per-profile register index and a trusted per-IP base. The first semantic
//! target is GC SCRATCH_REG0 because upstream AMDGPU uses it for explicit
//! write/readback ring tests on GFX12. Titanweave deliberately does NOT guess
//! the generated register index or Navi48 IP base: until those are imported
//! from trusted discovery/generated data, the live store is deferred.
//!
//! If promoted on a future exact target, C16 performs only an identity write:
//! read original u32 -> write the same u32 -> bounded readback -> require exact
//! equality -> recheck PCI bus mastering is still OFF. One restore write of the
//! original value is the only rollback. Firmware upload, command submission,
//! and Radeon bus-master enable remain forbidden.

use crate::{memory::FrameAllocator,native_gpu_c9,native_gpu_c14,native_gpu_c15,native_gpu_binding,paging,pci,serial,sync::SpinLock};

pub const K14C16_ABI_VERSION:u32=1;
pub const RADEON_C16_MMIO_IDENTITY_WRITE_ALLOWED:bool=true;
pub const RADEON_C16_FIRMWARE_UPLOAD_ALLOWED:bool=false;
pub const RADEON_C16_COMMAND_SUBMIT_ALLOWED:bool=false;
pub const RADEON_C16_BUS_MASTER_ALLOWED:bool=false;
pub const RADEON_C16_MAX_READBACK_POLLS:u8=32;
pub const RADEON_C16_MAX_MMIO_WRITES:u8=2;
pub const RADEON_C16_MMIO_BAR_INDEX:u8=5;
pub const RADEON_C16_PAGE_BYTES:u64=4096;

#[derive(Clone,Copy,Debug,PartialEq,Eq)] #[repr(u8)]
pub enum ReviewedTarget { None=0, Gfx12GcScratchReg0=1 }

#[derive(Clone,Copy,Debug)]
pub struct C16State{
 pub amd_present:bool,pub c14_ready:bool,pub c15_ready:bool,pub target_reviewed:bool,pub target_resolved:bool,
 pub trusted_base_ready:bool,pub bar5_ready:bool,pub transaction_attempted:bool,pub transaction_verified:bool,
 pub rollback_attempted:bool,pub rollback_verified:bool,pub bus_master_before_off:bool,pub bus_master_after_off:bool,
 pub writes_performed:u8,pub readback_polls:u8,pub value_before:u32,pub value_after:u32,pub target:ReviewedTarget,
 pub transaction_fingerprint:u64,pub device_id:u16,pub revision:u8,pub firmware_upload_enabled:bool,
 pub command_submit_enabled:bool,pub radeon_bus_master_enabled:bool,pub fallback_armed:bool,
}
impl C16State{pub const EMPTY:Self=Self{amd_present:false,c14_ready:false,c15_ready:false,target_reviewed:false,target_resolved:false,
 trusted_base_ready:false,bar5_ready:false,transaction_attempted:false,transaction_verified:false,rollback_attempted:false,
 rollback_verified:false,bus_master_before_off:false,bus_master_after_off:false,writes_performed:0,readback_polls:0,
 value_before:0,value_after:0,target:ReviewedTarget::None,transaction_fingerprint:0,device_id:0,revision:0,
 firmware_upload_enabled:false,command_submit_enabled:false,radeon_bus_master_enabled:false,fallback_armed:true};}
static STATE:SpinLock<C16State>=SpinLock::new(C16State::EMPTY);

fn bar5()->Option<u64>{let b=native_gpu_binding::state();pci::memory_bar_base(crate::pci::PciFunction{bus:b.selected_bus,device:b.selected_device,function:b.selected_function,vendor_id:0,device_id:0,class_code:0,subclass:0,programming_interface:0,revision:0,header_type:0},RADEON_C16_MMIO_BAR_INDEX)}
fn fingerprint(d:u16,r:u8,b:u32,a:u32,w:u8)->u64{let mut x=0x4331_3652_4d4d_494fu64;x^=(d as u64)<<40;x^=(r as u64)<<32;x^=b as u64;x=x.rotate_left(13)^(a as u64);x.rotate_left(9)^w as u64}

unsafe fn identity_write_u32_page(allocator:&mut FrameAllocator<'_>,kernel_cr3:u64,bar:u64,byte_offset:u64)->Result<(u32,u32,u8,u8),&'static str>{
 if byte_offset>0x00ff_ffff || byte_offset&3!=0{return Err("K14.C16 invalid MMIO register offset");}
 let page_off=byte_offset&!(RADEON_C16_PAGE_BYTES-1);let in_page=byte_offset&(RADEON_C16_PAGE_BYTES-1);
 if in_page+4>RADEON_C16_PAGE_BYTES{return Err("K14.C16 cross-page register target");}
 let phys=bar.checked_add(page_off).ok_or("K14.C16 MMIO physical overflow")?;
 let virt=paging::map_kernel_mmio(allocator,kernel_cr3,phys,RADEON_C16_PAGE_BYTES)?;
 let p=(virt+in_page) as *mut u32;let before=unsafe{core::ptr::read_volatile(p)};
 unsafe{core::ptr::write_volatile(p,before)};let mut polls=0u8;let mut after=0u32;
 while polls<RADEON_C16_MAX_READBACK_POLLS{after=unsafe{core::ptr::read_volatile(p)};polls+=1;if after==before{return Ok((before,after,1,polls));}}
 unsafe{core::ptr::write_volatile(p,before)};let restored=unsafe{core::ptr::read_volatile(p)};
 if restored!=before{return Err("K14.C16 MMIO identity-write readback and rollback failed");}
 Err("K14.C16 MMIO identity-write required rollback; transaction not qualified")
}

fn self_test()->Result<(),&'static str>{if K14C16_ABI_VERSION!=1||!RADEON_C16_MMIO_IDENTITY_WRITE_ALLOWED||RADEON_C16_FIRMWARE_UPLOAD_ALLOWED||RADEON_C16_COMMAND_SUBMIT_ALLOWED||RADEON_C16_BUS_MASTER_ALLOWED||RADEON_C16_MAX_READBACK_POLLS==0||RADEON_C16_MAX_MMIO_WRITES!=2||RADEON_C16_MMIO_BAR_INDEX!=5{return Err("K14.C16 fail-closed constants invalid");}Ok(())}

pub fn initialize(_allocator:&mut FrameAllocator<'_>,_kernel_cr3:u64)->Result<C16State,&'static str>{
 self_test()?;let c9=native_gpu_c9::state();let c14=native_gpu_c14::state();let c15=native_gpu_c15::state();let mut s=C16State{amd_present:c9.amd_present,c14_ready:c14.write_prerequisites_complete,c15_ready:!c9.amd_present||c15.identity_write_verified,device_id:c9.device_id,revision:c9.revision,..C16State::EMPTY};
 if c9.profile==native_gpu_c9::ProfileId::Navi48Rx9070{s.target=ReviewedTarget::Gfx12GcScratchReg0;s.target_reviewed=true;s.target_resolved=false;s.trusted_base_ready=false;}
 s.bar5_ready=bar5().is_some();
 serial::println(format_args!("[C16RV] reviewed MMIO target: gfx12_gc_scratch_reg0=true upstream_write_readback_semantics=true exact_generated_index_imported=false guessed_offsets=false identity_write_only=true"));
 serial::println(format_args!("[C16PG] MMIO-write policy: require=C14_ready+C15_write_framework+reviewed_target+exact_register_index+trusted_ip_base+BAR5+bus_master_off; identity_write=true max_polls={} max_writes={} firmware=false submit=false bus_master_enable=false",RADEON_C16_MAX_READBACK_POLLS,RADEON_C16_MAX_MMIO_WRITES));
 if !s.amd_present{serial::println(format_args!("[C16HW] Radeon MMIO identity-write: present=false qemu_deferred=true target=none attempted=false verified=false fallback=true"));}
 else {
  let b=native_gpu_binding::state();let cmd=pci::read_u16(b.selected_bus,b.selected_device,b.selected_function,0x04);s.bus_master_before_off=cmd&(1<<2)==0;
  if !s.bus_master_before_off{return Err("K14.C16 Radeon bus mastering unexpectedly enabled");}
  if !(s.c14_ready&&s.c15_ready){serial::println(format_args!("[C16HW] Radeon MMIO identity-write: present=true devid={:#06x} prerequisites=false attempted=false reason=prior_gate_not_qualified fallback=true",s.device_id));}
  else if !s.target_reviewed||!s.target_resolved||!s.trusted_base_ready{serial::println(format_args!("[C16HW] Radeon MMIO identity-write: present=true devid={:#06x} target={:?} reviewed={} resolved={} trusted_base={} attempted=false reason=exact_target_or_base_unresolved fallback=true",s.device_id,s.target,s.target_reviewed,s.target_resolved,s.trusted_base_ready));}
  else {return Err("K14.C16 live MMIO executor reached without an imported exact target; fail closed");}
  let cmd2=pci::read_u16(b.selected_bus,b.selected_device,b.selected_function,0x04);s.bus_master_after_off=cmd2&(1<<2)==0;if !s.bus_master_after_off{return Err("K14.C16 bus mastering changed during deferred MMIO gate");}
 }
 if s.transaction_verified&&(!s.transaction_attempted||!s.target_reviewed||!s.target_resolved||!s.trusted_base_ready||!s.bus_master_before_off||!s.bus_master_after_off){return Err("K14.C16 MMIO transaction qualified without all gates");}
 if s.writes_performed>RADEON_C16_MAX_MMIO_WRITES||s.firmware_upload_enabled||s.command_submit_enabled||s.radeon_bus_master_enabled{return Err("K14.C16 destructive capability promoted early");}
 serial::println(format_args!("[C16RD] K14.C16 reviewed MMIO-write gate ready: amd_present={} C14_ready={} C15_ready={} target={:?} reviewed={} resolved={} trusted_base={} bar5={} attempted={} verified={} writes={} polls={} bus_master_before_off={} bus_master_after_off={} fingerprint={:#018x} fallback=true",s.amd_present,s.c14_ready,s.c15_ready,s.target,s.target_reviewed,s.target_resolved,s.trusted_base_ready,s.bar5_ready,s.transaction_attempted,s.transaction_verified,s.writes_performed,s.readback_polls,s.bus_master_before_off,s.bus_master_after_off,s.transaction_fingerprint));
 *STATE.lock()=s;Ok(s)
}
pub fn state()->C16State{*STATE.lock()}
pub fn packed_status()->u64{let s=state();let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24)|(u64::from(s.writes_performed)<<16);for(bit,on)in[s.amd_present,s.c14_ready,s.c15_ready,s.target_reviewed,s.target_resolved,s.trusted_base_ready,s.bar5_ready,s.transaction_attempted,s.transaction_verified,s.bus_master_before_off,s.bus_master_after_off,s.fallback_armed].into_iter().enumerate(){if on{v|=1u64<<bit;}}v}
