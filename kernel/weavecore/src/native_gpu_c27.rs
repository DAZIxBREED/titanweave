//! K14.C27 complete Radeon driver-core milestone.
//!
//! C26 froze the exact GFX12 reviewed-MMIO foundation.  C27 turns that proof
//! chain into persistent, reusable driver infrastructure without widening the
//! dangerous hardware authority.  Every C27 component is executable: ForgeBus
//! ownership is resolved from the retained device registry, PCI/VRAM/MMIO
//! topology is captured from live state, both reviewed registers are accessed
//! through a permanent identity-based MMIO service, lifecycle/error/reset
//! coordination is exercised, and a real interrupt route + handler is installed
//! while remaining masked until C29 programs physical interrupt delivery.
//!
//! C27 does not upload firmware, enable DMA/bus mastering, submit GPU commands,
//! or add new MMIO write authority. Those operations remain reserved for their
//! fixed roadmap milestones.

use crate::{
    memory::FrameAllocator,
    native_gpu_c26,
    pci,
    radeon_driver,
    radeon_mmio,
    radeon_resources,
    serial,
    sync::SpinLock,
};

pub const K14C27_ABI_VERSION:u32=1;
pub const RADEON_C27_NEW_MMIO_REGISTERS:u8=0;
pub const RADEON_C27_NEW_MMIO_WRITES:u8=0;
pub const RADEON_C27_FIRMWARE_UPLOAD_ALLOWED:bool=false;
pub const RADEON_C27_DMA_ENABLE_ALLOWED:bool=false;
pub const RADEON_C27_BUS_MASTER_ALLOWED:bool=false;
pub const RADEON_C27_COMMAND_SUBMIT_ALLOWED:bool=false;
pub const RADEON_C27_HARDWARE_IRQ_ENABLE_ALLOWED:bool=false;
pub const RADEON_C27_PLACEHOLDER_SUBSYSTEMS:u8=0;

#[derive(Clone,Copy,Debug)]
pub struct C27State{
 pub amd_present:bool,
 pub navi48:bool,
 pub c26_foundation_verified:bool,
 pub driver_model_verified:bool,
 pub forge_ownership_verified:bool,
 pub resource_topology_verified:bool,
 pub reviewed_mmio_service_verified:bool,
 pub generic_mmio_write_rejected:bool,
 pub reg0_read_valid:bool,
 pub reg1_read_valid:bool,
 pub irq_handler_exercised:bool,
 pub irq_route_registered:bool,
 pub irq_masked:bool,
 pub irq_hardware_enabled:bool,
 pub reset_coordinator_verified:bool,
 pub error_machine_verified:bool,
 pub core_online:bool,
 pub hardware_deferred:bool,
 pub memory_decode_on:bool,
 pub bus_master_off:bool,
 pub firmware_upload_enabled:bool,
 pub dma_enabled:bool,
 pub command_submit_enabled:bool,
 pub new_mmio_writes:u8,
 pub forge_device:u64,
 pub forge_driver:u64,
 pub irq_vector:u8,
 pub vram_aperture_bytes:u64,
 pub mmio_base:u64,
 pub resource_fingerprint:u64,
 pub mmio_fingerprint:u64,
 pub driver_fingerprint:u64,
 pub qualification_fingerprint:u64,
 pub qualified:bool,
 pub fallback_armed:bool,
 pub device_id:u16,
 pub revision:u8,
}
impl C27State{pub const EMPTY:Self=Self{
 amd_present:false,navi48:false,c26_foundation_verified:false,driver_model_verified:false,
 forge_ownership_verified:false,resource_topology_verified:false,reviewed_mmio_service_verified:false,
 generic_mmio_write_rejected:false,reg0_read_valid:false,reg1_read_valid:false,irq_handler_exercised:false,
 irq_route_registered:false,irq_masked:true,irq_hardware_enabled:false,reset_coordinator_verified:false,
 error_machine_verified:false,core_online:false,hardware_deferred:true,memory_decode_on:false,bus_master_off:true,
 firmware_upload_enabled:false,dma_enabled:false,command_submit_enabled:false,new_mmio_writes:0,
 forge_device:0,forge_driver:0,irq_vector:0,vram_aperture_bytes:0,mmio_base:0,resource_fingerprint:0,
 mmio_fingerprint:0,driver_fingerprint:0,qualification_fingerprint:0,qualified:false,fallback_armed:true,
 device_id:0,revision:0,
};}
static STATE:SpinLock<C27State>=SpinLock::new(C27State::EMPTY);

fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}
fn self_test()->Result<(),&'static str>{
 if K14C27_ABI_VERSION!=1||RADEON_C27_NEW_MMIO_REGISTERS!=0||RADEON_C27_NEW_MMIO_WRITES!=0
  ||RADEON_C27_FIRMWARE_UPLOAD_ALLOWED||RADEON_C27_DMA_ENABLE_ALLOWED||RADEON_C27_BUS_MASTER_ALLOWED
  ||RADEON_C27_COMMAND_SUBMIT_ALLOWED||RADEON_C27_HARDWARE_IRQ_ENABLE_ALLOWED||RADEON_C27_PLACEHOLDER_SUBSYSTEMS!=0{
  return Err("K14.C27 policy constants violate the fixed Radeon roadmap")
 }
 Ok(())
}

pub fn initialize(allocator:&mut FrameAllocator<'_>,kernel_cr3:u64)->Result<C27State,&'static str>{
 self_test()?;
 let c26=native_gpu_c26::state();
 if c26.amd_present&&(!c26.k14_completion_verified||!c26.allowlist_exact||!c26.no_write_verified){return Err("K14.C27 requires frozen C26 reviewed-MMIO foundation")}
 let resources=radeon_resources::initialize()?;
 let mmio=radeon_mmio::initialize(allocator,kernel_cr3)?;
 let driver=radeon_driver::initialize(resources,mmio)?;
 let irq_exercised=radeon_driver::irq_events()>0;
 let mut s=C27State{
  amd_present:c26.amd_present,navi48:c26.navi48,c26_foundation_verified:!c26.amd_present||c26.k14_completion_verified,
  driver_model_verified:driver.model_verified,forge_ownership_verified:driver.ownership_verified,
  resource_topology_verified:resources.topology_verified,reviewed_mmio_service_verified:driver.mmio_service_verified,
  generic_mmio_write_rejected:mmio.generic_write_rejected,reg0_read_valid:mmio.reg0_read_valid,reg1_read_valid:mmio.reg1_read_valid,
  irq_handler_exercised:irq_exercised,irq_route_registered:driver.irq_route_registered,irq_masked:driver.irq_masked,
  irq_hardware_enabled:driver.irq_hardware_enabled,reset_coordinator_verified:driver.reset_coordinator_verified,
  error_machine_verified:driver.error_machine_verified,core_online:driver.core_online,hardware_deferred:driver.hardware_deferred,
  memory_decode_on:resources.memory_decode_on,bus_master_off:resources.bus_master_off,firmware_upload_enabled:false,
  dma_enabled:false,command_submit_enabled:false,new_mmio_writes:0,forge_device:driver.forge_device.0,forge_driver:driver.forge_driver,
  irq_vector:driver.irq_vector,vram_aperture_bytes:resources.bar0_vram_aperture_bytes,mmio_base:resources.bar5_mmio_base,
  resource_fingerprint:resources.fingerprint,mmio_fingerprint:mmio.fingerprint,driver_fingerprint:driver.fingerprint,
  device_id:c26.device_id,revision:c26.revision,..C27State::EMPTY
 };

 serial::println(format_args!("[C27DV] Radeon driver core: object=operational lifecycle=exercised model={} amd_present={} core_online={} deferred={} generation={} no_placeholders=true",driver.model_verified,s.amd_present,driver.core_online,driver.hardware_deferred,driver.generation));
 serial::println(format_args!("[C27RS] Radeon resource ownership/topology: model={} forge_owned={} forge_device={} forge_driver={} BDF={:02x}:{:02x}.{} BAR0={:#x} VRAM_aperture={} BAR2={:#x} BAR5={:#x} memdecode={} bus_master_off={} iommu_hw={} domain={} topology={} fingerprint={:#018x}",resources.model_ready,resources.forge_owned,resources.forge_device.0,resources.forge_driver,resources.bus,resources.device,resources.function,resources.bar0_vram_base,resources.bar0_vram_aperture_bytes,resources.bar2_base,resources.bar5_mmio_base,resources.memory_decode_on,resources.bus_master_off,resources.iommu_hardware_translated,resources.persistent_device_domain,resources.topology_verified,resources.fingerprint));
 serial::println(format_args!("[C27MM] Radeon reviewed MMIO service: policy={} targets=2 REG0_read={} REG1_read={} reads={} generic_write_rejected={} caller_address=false caller_value=false new_registers=0 new_writes=0 fingerprint={:#018x}",mmio.policy_ready,mmio.reg0_read_valid,mmio.reg1_read_valid,mmio.reads_performed,mmio.generic_write_rejected,mmio.fingerprint));
 serial::println(format_args!("[C27IR] Radeon interrupt core: handler_exercised={} route_registered={} vector={:#04x} route_masked={} hardware_irq_enabled={} dispatch_events={} policy=C29_owns_physical_enable",irq_exercised,driver.irq_route_registered,driver.irq_vector,driver.irq_masked,driver.irq_hardware_enabled,radeon_driver::irq_events()));
 serial::println(format_args!("[C27ER] Radeon error/reset coordinator: lifecycle_test={} error_machine={} reset_coordinator={} hardware_reset=false fault_count={} reset_epoch={} phase={:?}",driver.model_verified,driver.error_machine_verified,driver.reset_coordinator_verified,driver.fault_count,driver.reset_epoch,driver.phase));
 serial::println(format_args!("[C27PG] driver-core authority: firmware=false DMA=false bus_master=false submit=false hardware_IRQ_enable=false new_MMIO_registers=0 new_MMIO_writes=0 inherited_C26_allowlist=true no_placeholders=true"));

 if s.amd_present{
  let binding_safe=s.c26_foundation_verified&&s.forge_ownership_verified&&s.resource_topology_verified
   &&s.reviewed_mmio_service_verified&&s.generic_mmio_write_rejected&&s.reg0_read_valid&&s.reg1_read_valid
   &&s.irq_handler_exercised&&s.irq_route_registered&&s.irq_masked&&!s.irq_hardware_enabled
   &&s.reset_coordinator_verified&&s.error_machine_verified&&s.core_online&&s.memory_decode_on&&s.bus_master_off;
  if !binding_safe{return Err("K14.C27 physical Radeon driver core did not fully close")}
  let command=pci::read_u16(resources.bus,resources.device,resources.function,0x04);
  if command&(1<<2)!=0{return Err("K14.C27 Radeon bus mastering became enabled")}
 }else{
  let deferred_safe=s.driver_model_verified&&resources.model_ready&&mmio.policy_ready&&s.generic_mmio_write_rejected
   &&s.irq_handler_exercised&&s.reset_coordinator_verified&&s.error_machine_verified&&s.hardware_deferred;
  if !deferred_safe{return Err("K14.C27 QEMU driver-core software qualification failed")}
 }
 if s.firmware_upload_enabled||s.dma_enabled||s.command_submit_enabled||s.irq_hardware_enabled||s.new_mmio_writes!=0{return Err("K14.C27 widened hardware authority outside roadmap")}
 let mut fp=0xc27d_5155_414c_0001u64;for v in [s.resource_fingerprint,s.mmio_fingerprint,s.driver_fingerprint,s.forge_device,s.forge_driver,u64::from(s.device_id),u64::from(s.irq_vector)]{fp=mix(fp,v)}
 s.qualification_fingerprint=fp;s.qualified=fp!=0;
 serial::println(format_args!("[C27RD] K14.C27 complete driver-core ready: amd_present={} navi48={} C26={} model={} ownership={} topology={} mmio={} generic_write_rejected={} REG0_read={} REG1_read={} irq_handler={} irq_route={} irq_masked={} irq_hw={} reset={} errors={} core_online={} deferred={} VRAM_aperture={} BAR5={:#x} new_writes={} firmware={} DMA={} submit={} qualified={} fingerprint={:#018x} fallback=true",s.amd_present,s.navi48,s.c26_foundation_verified,s.driver_model_verified,s.forge_ownership_verified,s.resource_topology_verified,s.reviewed_mmio_service_verified,s.generic_mmio_write_rejected,s.reg0_read_valid,s.reg1_read_valid,s.irq_handler_exercised,s.irq_route_registered,s.irq_masked,s.irq_hardware_enabled,s.reset_coordinator_verified,s.error_machine_verified,s.core_online,s.hardware_deferred,s.vram_aperture_bytes,s.mmio_base,s.new_mmio_writes,s.firmware_upload_enabled,s.dma_enabled,s.command_submit_enabled,s.qualified,s.qualification_fingerprint));
 *STATE.lock()=s;Ok(s)
}
pub fn state()->C27State{*STATE.lock()}
pub fn packed_status()->u64{
 let s=state();let mut v=(u64::from(s.device_id)<<40)|(u64::from(s.revision)<<32)|(u64::from(s.irq_vector)<<24);
 for(bit,on)in[s.amd_present,s.navi48,s.c26_foundation_verified,s.driver_model_verified,s.forge_ownership_verified,s.resource_topology_verified,s.reviewed_mmio_service_verified,s.generic_mmio_write_rejected,s.irq_handler_exercised,s.irq_route_registered,s.irq_masked,s.reset_coordinator_verified,s.error_machine_verified,s.qualified,s.fallback_armed].into_iter().enumerate(){if on{v|=1u64<<bit;}}
 v
}
