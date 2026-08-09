//! K14.C6 live AMD-Vi hardware-programming boundary.
//!
//! C6 consumes the concrete C5 table image and introduces the physical AMD-Vi
//! register-programming path.  Automatic activation remains fail-closed until
//! a bare-metal AMD host proves the exact IVHD unit and Radeon requester.  The
//! programming primitive exists now; QEMU qualification verifies that it can
//! never be promoted by the Intel VT-d surrogate path.

use crate::{
    amd_vi::{self,AmdViHardwarePlan}, k11_backends, memory::FrameAllocator,
    native_gpu::NativeGpuVendor, native_gpu_binding, native_gpu_c5, paging,
    serial, sync::SpinLock,
};

pub const K14C6_ABI_VERSION:u32=1;
pub const RADEON_C6_DOMAIN_ID:u16=0x14c5; // same persistent C5 domain
pub const AMDVI_C6_ENABLE_BARE_METAL_PROGRAMMING:bool=false;
pub const RADEON_C6_MMIO_WRITE_ALLOWED:bool=false;
pub const RADEON_C6_FIRMWARE_UPLOAD_ALLOWED:bool=false;
pub const RADEON_C6_COMMAND_SUBMIT_ALLOWED:bool=false;

#[derive(Clone,Copy,Debug)]
pub struct C6State{
 pub amd_present:bool,pub amd_vi_active:bool,pub c5_structures_ready:bool,
 pub exact_requester_bound:bool,pub register_base:u64,pub mmio_window_mapped:bool,
 pub hardware_programming_eligible:bool,pub hardware_programmed:bool,
 pub translation_enabled:bool,pub invalidation_path_ready:bool,pub fault_capture_ready:bool,
 pub persistent_domain_live:bool,pub read_only_radeon_mmio_promoted:bool,
 pub bus_master_enabled:bool,pub fallback_armed:bool,pub requester:u16,pub domain_id:u16,
}
impl C6State{pub const EMPTY:Self=Self{amd_present:false,amd_vi_active:false,c5_structures_ready:false,
 exact_requester_bound:false,register_base:0,mmio_window_mapped:false,hardware_programming_eligible:false,
 hardware_programmed:false,translation_enabled:false,invalidation_path_ready:false,fault_capture_ready:false,
 persistent_domain_live:false,read_only_radeon_mmio_promoted:false,bus_master_enabled:false,fallback_armed:true,
 requester:0,domain_id:RADEON_C6_DOMAIN_ID};}
static STATE:SpinLock<C6State>=SpinLock::new(C6State::EMPTY);

fn self_test()->Result<(),&'static str>{
 if K14C6_ABI_VERSION!=1 || RADEON_C6_DOMAIN_ID==0 || AMDVI_C6_ENABLE_BARE_METAL_PROGRAMMING
   || RADEON_C6_MMIO_WRITE_ALLOWED || RADEON_C6_FIRMWARE_UPLOAD_ALLOWED || RADEON_C6_COMMAND_SUBMIT_ALLOWED {
   return Err("K14.C6 fail-closed constants invalid")
 }
 amd_vi::c6_register_self_test()?; Ok(())
}

pub fn initialize(allocator:&mut FrameAllocator<'_>,kernel_cr3:u64)->Result<C6State,&'static str>{
 self_test()?;
 let binding=native_gpu_binding::state(); let c5=native_gpu_c5::state();
 let amd_present=binding.selected_vendor==NativeGpuVendor::Amd as u8;
 let amd_vi_active=k11_backends::active_iommu()==k11_backends::ActiveIommu::AmdVi;
 let register_base=k11_backends::amd_primary_register_base().unwrap_or(0);
 let structures=c5.page_tables_ready&&c5.device_table_ready&&c5.command_buffer_ready&&c5.event_log_ready&&c5.fault_path_ready;
 let mut st=C6State{amd_present,amd_vi_active,c5_structures_ready:structures,exact_requester_bound:c5.exact_requester_bound,
  register_base,requester:c5.requester,..C6State::EMPTY};

 serial::println(format_args!("[C6RG] AMD-Vi live register engine: mmio_span={:#x} devtab={:#x} cmdbuf={:#x} evtlog={:#x} control={:#x} status={:#x}",
  amd_vi::AMDVI_MMIO_BYTES,amd_vi::AMDVI_REG_DEVICE_TABLE_BASE,amd_vi::AMDVI_REG_COMMAND_BUFFER_BASE,
  amd_vi::AMDVI_REG_EVENT_LOG_BASE,amd_vi::AMDVI_REG_CONTROL,amd_vi::AMDVI_REG_STATUS));
 serial::println(format_args!("[C6SQ] AMD-Vi activation sequence: exact_rid -> pinned_tables -> map_iommu_mmio -> publish_bases -> enable_cmd_event -> enable_translation -> invalidate_completion -> fault_check -> persistent_domain"));

 if !amd_present {
   serial::println(format_args!("[C6HW] live AMD-Vi programming: present=false qemu_deferred=true programmed=false translation=false domain_live=false read_mmio=false bus_master=false fallback=true"));
 } else if !amd_vi_active || register_base==0 {
   serial::println(format_args!("[C6HW] live AMD-Vi programming: present=true amd_vi=false programmed=false translation=false domain_live=false reason=no_qualified_amdvi_unit bus_master=false fallback=true"));
 } else if !structures || !c5.exact_requester_bound {
   serial::println(format_args!("[C6HW] live AMD-Vi programming: present=true amd_vi=true structures=false programmed=false translation=false domain_live=false reason=c5_exact_requester_image_incomplete bus_master=false fallback=true"));
 } else {
   st.hardware_programming_eligible=true;
   let plan=AmdViHardwarePlan{register_base,device_table:c5.device_table_phys,command_buffer:c5.command_buffer_phys,event_log:c5.event_log_phys,
      requester:crate::pci_address::RequesterId(c5.requester),domain:c5.domain_id};
   plan.validate()?;
   if AMDVI_C6_ENABLE_BARE_METAL_PROGRAMMING {
      let mmio=paging::map_kernel_mmio(allocator,kernel_cr3,register_base,amd_vi::AMDVI_MMIO_BYTES)?;
      st.mmio_window_mapped=true;
      unsafe{amd_vi::program_hardware_unit(mmio,plan)?;}
      st.hardware_programmed=true; st.translation_enabled=true;
      st.invalidation_path_ready=true; st.fault_capture_ready=true;
      st.persistent_domain_live=true; st.read_only_radeon_mmio_promoted=true;
      serial::println(format_args!("[C6HW] live AMD-Vi programming: present=true rid={:#06x} programmed=true translation=true domain_live=true read_mmio=true bus_master=false fallback=true",c5.requester));
   } else {
      serial::println(format_args!("[C6HW] live AMD-Vi programming: present=true rid={:#06x} eligible=true programmed=false translation=false domain_live=false reason=bare_metal_programming_gate_not_armed bus_master=false fallback=true",c5.requester));
   }
 }
 if st.persistent_domain_live && !(st.hardware_programmed&&st.translation_enabled&&st.invalidation_path_ready&&st.fault_capture_ready&&st.exact_requester_bound){return Err("K14.C6 domain promoted without complete hardware proof")}
 if st.bus_master_enabled||RADEON_C6_MMIO_WRITE_ALLOWED||RADEON_C6_FIRMWARE_UPLOAD_ALLOWED||RADEON_C6_COMMAND_SUBMIT_ALLOWED{return Err("K14.C6 destructive Radeon capability promoted early")}
 serial::println(format_args!("[C6RD] K14.C6 live AMD-Vi engine ready: amd_present={} amd_vi={} structures={} exact_bound={} eligible={} programmed={} translation={} invalidate={} fault={} domain_live={} read_mmio={} write_mmio=false firmware=false submit=false bus_master=false fallback=true",
 st.amd_present,st.amd_vi_active,st.c5_structures_ready,st.exact_requester_bound,st.hardware_programming_eligible,st.hardware_programmed,st.translation_enabled,st.invalidation_path_ready,st.fault_capture_ready,st.persistent_domain_live,st.read_only_radeon_mmio_promoted));
 *STATE.lock()=st; Ok(st)
}
pub fn state()->C6State{*STATE.lock()}
pub fn packed_status()->u64{let s=state();let mut v=(u64::from(s.requester)<<16)|(u64::from(s.domain_id)<<32);for(bit,on)in[
 s.amd_present,s.amd_vi_active,s.c5_structures_ready,s.exact_requester_bound,s.mmio_window_mapped,s.hardware_programming_eligible,
 s.hardware_programmed,s.translation_enabled,s.invalidation_path_ready,s.fault_capture_ready,s.persistent_domain_live,s.read_only_radeon_mmio_promoted,s.fallback_armed].into_iter().enumerate(){if on{v|=1u64<<bit}}v}
