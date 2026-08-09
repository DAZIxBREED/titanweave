//! K14.C27 Radeon resource ownership and topology capture.
//!
//! This is a live inventory backed by ForgeBus and PCI configuration space.  It
//! does not size BARs destructively or claim capabilities that C28 has not yet
//! implemented.  On physical Radeon hardware the exact retained ForgeBus owner,
//! driver, BAR bases, VRAM aperture size, IRQ line, IOMMU readiness and PCI
//! command state are captured into one immutable driver-core snapshot.

use crate::{
    device::{DeviceId, DeviceState},
    forgebus,
    native_gpu,
    native_gpu_binding,
    native_gpu_c19,
    native_gpu_c26,
    pci,
    sync::SpinLock,
};

pub const RADEON_RESOURCE_ABI_VERSION:u32=1;
pub const RADEON_RESOURCE_BAR_COUNT:u8=6;

#[derive(Clone,Copy,Debug)]
pub struct RadeonResourceState {
    pub model_ready:bool,
    pub amd_present:bool,
    pub hardware_deferred:bool,
    pub forge_owned:bool,
    pub forge_device:DeviceId,
    pub forge_driver:u64,
    pub forge_state:Option<DeviceState>,
    pub bus:u8,
    pub device:u8,
    pub function:u8,
    pub device_id:u16,
    pub revision:u8,
    pub bar0_vram_base:u64,
    pub bar0_vram_aperture_bytes:u64,
    pub bar2_base:u64,
    pub bar5_mmio_base:u64,
    pub bar0_ready:bool,
    pub bar5_ready:bool,
    pub irq_line:u8,
    pub memory_decode_on:bool,
    pub bus_master_off:bool,
    pub iommu_hardware_translated:bool,
    pub persistent_device_domain:bool,
    pub topology_verified:bool,
    pub fingerprint:u64,
}
impl RadeonResourceState { pub const EMPTY:Self=Self{
    model_ready:false,amd_present:false,hardware_deferred:true,forge_owned:false,
    forge_device:DeviceId(0),forge_driver:0,forge_state:None,bus:0,device:0,function:0,
    device_id:0,revision:0,bar0_vram_base:0,bar0_vram_aperture_bytes:0,bar2_base:0,
    bar5_mmio_base:0,bar0_ready:false,bar5_ready:false,irq_line:0,memory_decode_on:false,
    bus_master_off:false,iommu_hardware_translated:false,persistent_device_domain:false,
    topology_verified:false,fingerprint:0,
}; }
static STATE:SpinLock<RadeonResourceState>=SpinLock::new(RadeonResourceState::EMPTY);

fn selected_function()->Option<pci::PciFunction>{
 let b=native_gpu_binding::state(); let mut found=None;
 pci::enumerate(|f|{if f.bus==b.selected_bus&&f.device==b.selected_device&&f.function==b.selected_function{found=Some(f);}}); found
}
fn mix(mut h:u64,v:u64)->u64{h^=v;h=h.wrapping_mul(0x100000001b3);h}

fn self_test()->Result<(),&'static str>{
 if RADEON_RESOURCE_ABI_VERSION!=1||RADEON_RESOURCE_BAR_COUNT!=6{return Err("Radeon resource model constants invalid")}
 let mut s=RadeonResourceState::EMPTY;
 s.forge_device=DeviceId(7);s.forge_driver=9;s.bar0_vram_base=0x1000;s.bar5_mmio_base=0x5000;
 s.bar0_ready=s.bar0_vram_base!=0;s.bar5_ready=s.bar5_mmio_base!=0;s.forge_owned=s.forge_device.0!=0&&s.forge_driver!=0;
 if !s.forge_owned||!s.bar0_ready||!s.bar5_ready{return Err("Radeon resource ownership self-test failed")}
 Ok(())
}

pub fn initialize()->Result<RadeonResourceState,&'static str>{
 self_test()?;
 let c26=native_gpu_c26::state();
 let binding=native_gpu_binding::state();
 let c19=native_gpu_c19::state();
 let mut s=RadeonResourceState{model_ready:true,amd_present:c26.amd_present,hardware_deferred:!c26.amd_present,device_id:c26.device_id,revision:c26.revision,..RadeonResourceState::EMPTY};
 if !c26.amd_present{
  s.fingerprint=mix(0xc27b_5253_5243_0001,u64::from(RADEON_RESOURCE_BAR_COUNT));
  *STATE.lock()=s;return Ok(s)
 }
 let f=selected_function().ok_or("Radeon resource owner PCI function disappeared")?;
 if f.vendor_id!=0x1002||f.device_id!=c26.device_id{return Err("Radeon resource owner identity mismatch")}
 s.bus=f.bus;s.device=f.device;s.function=f.function;
 s.forge_device=forgebus::device_id_for_pci(f).ok_or("Radeon is not retained by ForgeBus")?;
 s.forge_driver=forgebus::driver_id_for_device(s.forge_device).ok_or("Radeon has no bound ForgeBus driver")?;
 s.forge_state=forgebus::device_state(s.forge_device);
 s.forge_owned=s.forge_device.0!=0&&s.forge_driver!=0&&s.forge_state.is_some();
 s.bar0_vram_base=pci::memory_bar_base(f,0).unwrap_or(0);
 s.bar2_base=pci::memory_bar_base(f,2).unwrap_or(0);
 s.bar5_mmio_base=pci::memory_bar_base(f,5).unwrap_or(0);
 s.bar0_ready=s.bar0_vram_base!=0;
 s.bar5_ready=s.bar5_mmio_base!=0;
 s.bar0_vram_aperture_bytes=c19.bar0_aperture_bytes;
 s.irq_line=pci::read_u8(f.bus,f.device,f.function,0x3c);
 let command=pci::read_u16(f.bus,f.device,f.function,0x04);
 s.memory_decode_on=command&(1<<1)!=0;
 s.bus_master_off=command&(1<<2)==0;
 s.iommu_hardware_translated=native_gpu::current_iommu_readiness()==native_gpu::NativeIommuReadiness::HardwareTranslated;
 s.persistent_device_domain=binding.persistent_device_domain;
 s.topology_verified=s.forge_owned&&s.bar5_ready&&s.memory_decode_on&&s.bus_master_off
   &&c26.allowlist_exact&&c26.k14_completion_verified;
 if !s.topology_verified{return Err("Radeon resource topology failed C27 ownership/safety verification")}
 let mut fp=0xc27b_544f_504f_0001u64;
 for v in [s.forge_device.0,s.forge_driver,s.bar0_vram_base,s.bar0_vram_aperture_bytes,s.bar2_base,s.bar5_mmio_base,u64::from(s.irq_line),u64::from(s.device_id),u64::from(s.revision)]{fp=mix(fp,v);}
 s.fingerprint=fp;s.hardware_deferred=false;
 *STATE.lock()=s;Ok(s)
}
pub fn state()->RadeonResourceState{*STATE.lock()}
