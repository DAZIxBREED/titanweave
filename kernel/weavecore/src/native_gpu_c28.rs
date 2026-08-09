//! K14.C28 large integrated Radeon memory + firmware + recovery milestone.
//!
//! This closes three real driver subsystems at once: reclaimable GTT backing and
//! BAR0 VRAM reservations, AMD common-firmware validation plus owned GTT staging,
//! and an executable watchdog/resource-safe software recovery path.  It does not
//! cross C29's authority boundary: GPU page tables, bus mastering, DMA engines,
//! command rings/queues and physical interrupt delivery remain disabled.

use core::ptr;
use crate::{
 memory::{FrameAllocator,FRAME_SIZE},native_gpu_c27,pci,radeon_driver,radeon_firmware,radeon_memory,
 radeon_recovery,radeon_resources,serial,sync::SpinLock,
};

pub const K14C28_ABI_VERSION:u32=1;
pub const RADEON_C28_PERSISTENT_GTT_BYTES:u64=16*1024;
pub const RADEON_C28_VRAM_PROBE_BYTES:u64=64*1024;
pub const RADEON_C28_NEW_COMMAND_SUBMISSION:bool=false;
pub const RADEON_C28_GPU_PAGE_TABLES:bool=false;
pub const RADEON_C28_DMA_ENGINES:bool=false;
pub const RADEON_C28_BUS_MASTER_ENABLE:bool=false;
pub const RADEON_C28_PHYSICAL_IRQ_ENABLE:bool=false;
pub const RADEON_C28_FIRMWARE_GPU_UPLOAD:bool=false;
pub const RADEON_C28_PLACEHOLDER_SUBSYSTEMS:u8=0;

#[derive(Clone,Copy,Debug)]
pub struct C28State{
 pub amd_present:bool,pub navi48:bool,pub c27_verified:bool,pub memory_initialized:bool,pub gtt_operational:bool,
 pub gtt_reclaim_verified:bool,pub gtt_cpu_mapping_verified:bool,pub persistent_gtt_verified:bool,pub persistent_gtt_object:u64,
 pub vram_allocator_ready:bool,pub vram_reservation_verified:bool,pub gpu_va_allocator_ready:bool,pub gpu_va_reservation_verified:bool,pub persistent_gpu_va:u64,pub gtt_budget:u64,pub vram_aperture_bytes:u64,
 pub firmware_parser_verified:bool,pub firmware_crc_verified:bool,pub firmware_scan_complete:bool,pub firmware_files_found:u8,
 pub firmware_files_staged:u8,pub firmware_staging_verified:bool,pub firmware_staged_bytes:u64,pub firmware_gpu_upload_performed:bool,
 pub watchdog_verified:bool,pub watchdog_registered:bool,pub recovery_lifecycle_verified:bool,pub recovery_resource_reclaim_ready:bool,pub recovery_interrupt_route_active:bool,
 pub physical_asic_reset_performed:bool,pub memory_decode_on:bool,pub bus_master_off:bool,pub gpu_page_tables_installed:bool,
 pub dma_enabled:bool,pub command_submit_enabled:bool,pub physical_irq_enabled:bool,pub hardware_deferred:bool,pub qualified:bool,
 pub memory_fingerprint:u64,pub firmware_fingerprint:u64,pub recovery_fingerprint:u64,pub qualification_fingerprint:u64,
 pub device_id:u16,pub revision:u8,pub fallback_armed:bool,
}
impl C28State{pub const EMPTY:Self=Self{amd_present:false,navi48:false,c27_verified:false,memory_initialized:false,gtt_operational:false,gtt_reclaim_verified:false,gtt_cpu_mapping_verified:false,persistent_gtt_verified:false,persistent_gtt_object:0,vram_allocator_ready:false,vram_reservation_verified:false,gpu_va_allocator_ready:false,gpu_va_reservation_verified:false,persistent_gpu_va:0,gtt_budget:0,vram_aperture_bytes:0,firmware_parser_verified:false,firmware_crc_verified:false,firmware_scan_complete:false,firmware_files_found:0,firmware_files_staged:0,firmware_staging_verified:false,firmware_staged_bytes:0,firmware_gpu_upload_performed:false,watchdog_verified:false,watchdog_registered:false,recovery_lifecycle_verified:false,recovery_resource_reclaim_ready:false,recovery_interrupt_route_active:false,physical_asic_reset_performed:false,memory_decode_on:false,bus_master_off:true,gpu_page_tables_installed:false,dma_enabled:false,command_submit_enabled:false,physical_irq_enabled:false,hardware_deferred:true,qualified:false,memory_fingerprint:0,firmware_fingerprint:0,recovery_fingerprint:0,qualification_fingerprint:0,device_id:0,revision:0,fallback_armed:true};}
static STATE:SpinLock<C28State>=SpinLock::new(C28State::EMPTY);
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}

fn self_test()->Result<(),&'static str>{
 if K14C28_ABI_VERSION!=1||RADEON_C28_NEW_COMMAND_SUBMISSION||RADEON_C28_GPU_PAGE_TABLES||RADEON_C28_DMA_ENGINES
  ||RADEON_C28_BUS_MASTER_ENABLE||RADEON_C28_PHYSICAL_IRQ_ENABLE||RADEON_C28_FIRMWARE_GPU_UPLOAD||RADEON_C28_PLACEHOLDER_SUBSYSTEMS!=0{
  return Err("K14.C28 policy violates locked K14 roadmap")
 }
 Ok(())
}

pub fn initialize(allocator:&mut FrameAllocator<'_>,kernel_cr3:u64)->Result<C28State,&'static str>{
 self_test()?;let c27=native_gpu_c27::state();if !c27.qualified{return Err("K14.C28 requires frozen qualified C27 driver core")}
 let resources=radeon_resources::state();let memory=radeon_memory::initialize(allocator,kernel_cr3,resources)?;
 let owner=if c27.forge_device!=0{c27.forge_device}else{0xc028};
 // Keep a real pinned control/journal allocation alive after C28 qualification.
 let control=radeon_memory::allocate_gtt(allocator,owner,RADEON_C28_PERSISTENT_GTT_BYTES,FRAME_SIZE)?;
 unsafe{for i in 0..256usize{ptr::write_volatile((control.kernel_virtual as *mut u8).add(i),(i as u8).wrapping_mul(17)^0x28)}for i in 0..256usize{if ptr::read_volatile((control.kernel_virtual as *const u8).add(i))!=((i as u8).wrapping_mul(17)^0x28){return Err("C28 persistent GTT control buffer readback failed")}}}
 radeon_memory::pin(owner,control.id,true)?;
 let mut vram_reservation_verified=!c27.amd_present;
 if c27.amd_present{
  let v=radeon_memory::reserve_vram(owner,RADEON_C28_VRAM_PROBE_BYTES,64*1024)?;
  if v.vram_address==0||v.mapped_bytes<RADEON_C28_VRAM_PROBE_BYTES{return Err("C28 VRAM reservation returned invalid range")}
  radeon_memory::free(allocator,owner,v.id)?;vram_reservation_verified=true;
 }
 let firmware=radeon_firmware::initialize(allocator,owner,c27.amd_present)?;
 let recovery=radeon_recovery::initialize()?;
 let (vram_used,gtt_used,gpu_va_used,active)=radeon_memory::usage();
 let mut s=C28State{amd_present:c27.amd_present,navi48:c27.navi48,c27_verified:c27.qualified,memory_initialized:memory.initialized,
  gtt_operational:memory.gtt_operational,gtt_reclaim_verified:memory.gtt_reclaim_verified,gtt_cpu_mapping_verified:memory.gtt_cpu_mapping_verified,
  persistent_gtt_verified:control.active&&control.kernel_virtual!=0&&active>0,persistent_gtt_object:control.id,vram_allocator_ready:memory.vram_allocator_ready,
  vram_reservation_verified,gpu_va_allocator_ready:memory.gpu_va_allocator_ready,gpu_va_reservation_verified:memory.gpu_va_reservation_verified,persistent_gpu_va:control.gpu_virtual,gtt_budget:memory.gtt_budget,vram_aperture_bytes:memory.vram_bytes,firmware_parser_verified:firmware.parser_verified,
  firmware_crc_verified:firmware.crc_verified,firmware_scan_complete:firmware.vfs_scan_complete,firmware_files_found:firmware.files_found,
  firmware_files_staged:firmware.files_staged,firmware_staging_verified:firmware.staging_verified,firmware_staged_bytes:firmware.staged_bytes,
  firmware_gpu_upload_performed:firmware.gpu_upload_performed,watchdog_verified:recovery.watchdog_actions_verified,
  watchdog_registered:recovery.watchdog_registered,recovery_lifecycle_verified:recovery.lifecycle_recovery_verified,
  recovery_resource_reclaim_ready:recovery.resource_reclaim_ready,recovery_interrupt_route_active:recovery.interrupt_route_active,physical_asic_reset_performed:recovery.physical_asic_reset_performed,
  memory_decode_on:resources.memory_decode_on,bus_master_off:resources.bus_master_off,gpu_page_tables_installed:memory.gpu_page_tables_installed,
  dma_enabled:memory.device_dma_enabled,command_submit_enabled:false,physical_irq_enabled:radeon_driver::state().irq_hardware_enabled,
  hardware_deferred:c27.hardware_deferred,memory_fingerprint:memory.fingerprint,firmware_fingerprint:firmware.digest_fingerprint,
  recovery_fingerprint:recovery.fingerprint,device_id:c27.device_id,revision:c27.revision,..C28State::EMPTY};
 serial::println(format_args!("[C28ME] Radeon memory manager: initialized={} GTT_operational={} reclaim={} CPU_map={} persistent={} object={} GPU_VA={:#x} active={} GTT_used={} GTT_budget={} GPU_VA_ready={} GPU_VA_reserved={} GPU_VA_used={} VRAM_ready={} VRAM_reservation={} VRAM_used={} VRAM_aperture={} GPU_page_tables=false device_DMA=false fingerprint={:#018x}",s.memory_initialized,s.gtt_operational,s.gtt_reclaim_verified,s.gtt_cpu_mapping_verified,s.persistent_gtt_verified,s.persistent_gtt_object,s.persistent_gpu_va,active,gtt_used,s.gtt_budget,s.gpu_va_allocator_ready,s.gpu_va_reservation_verified,gpu_va_used,s.vram_allocator_ready,s.vram_reservation_verified,vram_used,s.vram_aperture_bytes,s.memory_fingerprint));
 serial::println(format_args!("[C28FW] Radeon firmware manager: parser={} CRC32={} VFS_scan={} found={} staged={} staging_verified={} staged_bytes={} GPU_upload=false digest_fingerprint={:#018x}",s.firmware_parser_verified,s.firmware_crc_verified,s.firmware_scan_complete,s.firmware_files_found,s.firmware_files_staged,s.firmware_staging_verified,s.firmware_staged_bytes,s.firmware_fingerprint));
 serial::println(format_args!("[C28RC] Radeon recovery manager: watchdog={} registered={} lifecycle={} reclaim_ready={} interrupt_fencing={} route_active={} physical_ASIC_reset=false fingerprint={:#018x}",s.watchdog_verified,s.watchdog_registered,s.recovery_lifecycle_verified,s.recovery_resource_reclaim_ready,recovery.interrupt_fencing_ready,s.recovery_interrupt_route_active,s.recovery_fingerprint));
 serial::println(format_args!("[C28PG] memory/firmware/recovery authority: C27=true GPU_VA_reservations=true GPU_page_tables=false DMA=false bus_master=false submit=false recovery_IRQ_route=active_if_physical physical_GPU_IRQ_programming=false firmware_GPU_upload=false physical_ASIC_reset=false no_placeholders=true C29_owns_execution=true"));
 let common=s.c27_verified&&s.memory_initialized&&s.gtt_operational&&s.gtt_reclaim_verified&&s.gtt_cpu_mapping_verified&&s.persistent_gtt_verified
  &&s.vram_reservation_verified&&s.gpu_va_allocator_ready&&s.gpu_va_reservation_verified&&s.persistent_gpu_va!=0&&s.firmware_parser_verified&&s.firmware_crc_verified&&s.firmware_scan_complete&&s.firmware_staging_verified
  &&s.watchdog_verified&&s.recovery_lifecycle_verified&&s.recovery_resource_reclaim_ready&&!s.firmware_gpu_upload_performed
  &&!s.physical_asic_reset_performed&&!s.gpu_page_tables_installed&&!s.dma_enabled&&!s.command_submit_enabled&&!s.physical_irq_enabled;
 if s.amd_present{
  if !common||!s.vram_allocator_ready||!s.recovery_interrupt_route_active||s.firmware_files_staged==0||!s.memory_decode_on||!s.bus_master_off{return Err("K14.C28 physical Radeon memory/firmware/recovery gates did not close")}
  let r=radeon_driver::state();let command=pci::read_u16(resources.bus,resources.device,resources.function,0x04);if command&(1<<2)!=0||r.irq_hardware_enabled{return Err("K14.C28 crossed C29 hardware authority boundary")}
 }else if !common||!s.hardware_deferred{return Err("K14.C28 QEMU memory/firmware/recovery qualification failed")}
 let mut fp=0xc28d_5155_414c_0001u64;for v in [s.memory_fingerprint,s.firmware_fingerprint,s.recovery_fingerprint,s.persistent_gtt_object,s.persistent_gpu_va,s.gtt_budget,s.vram_aperture_bytes,u64::from(s.device_id),u64::from(s.firmware_files_staged)]{fp=mix(fp,v)}s.qualification_fingerprint=fp;s.qualified=fp!=0;
 serial::println(format_args!("[C28RD] K14.C28 memory+firmware+recovery ready: amd_present={} navi48={} C27={} GTT={} persistent={} VRAM={} firmware_parser={} firmware_staged={} staged_files={} watchdog={} recovery={} bus_master_off={} DMA={} submit={} IRQ_hw={} qualified={} fingerprint={:#018x} fallback=true",s.amd_present,s.navi48,s.c27_verified,s.gtt_operational,s.persistent_gtt_verified,s.vram_reservation_verified,s.firmware_parser_verified,s.firmware_staging_verified,s.firmware_files_staged,s.watchdog_verified,s.recovery_lifecycle_verified,s.bus_master_off,s.dma_enabled,s.command_submit_enabled,s.physical_irq_enabled,s.qualified,s.qualification_fingerprint));
 *STATE.lock()=s;Ok(s)
}
pub fn state()->C28State{*STATE.lock()}
pub fn packed_status()->u64{let s=state();let mut v=(u64::from(s.device_id)<<40)|(u64::from(s.revision)<<32)|(u64::from(s.firmware_files_staged)<<24);for(bit,on)in[s.amd_present,s.navi48,s.c27_verified,s.memory_initialized,s.gtt_operational,s.gtt_reclaim_verified,s.persistent_gtt_verified,s.vram_reservation_verified,s.firmware_parser_verified,s.firmware_crc_verified,s.firmware_staging_verified,s.watchdog_verified,s.recovery_lifecycle_verified,s.qualified,s.fallback_armed].into_iter().enumerate(){if on{v|=1u64<<bit}}v}
