//! K11.1-K11.8 integrated backend discovery and retained IOMMU runtime.
use crate::{acpi::AcpiCatalog,amd_vi::AmdVi,intel_vtd::IntelVtd,k11_stress,serial,sync::SpinLock};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum ActiveIommu{None,AmdVi,IntelVtd}
pub struct BackendRuntime{pub active:ActiveIommu,pub amd:AmdVi,pub intel:IntelVtd,pub stress_passed:u32,pub stress_failed:u32,pub initialized:bool}
impl BackendRuntime{pub const fn new()->Self{Self{active:ActiveIommu::None,amd:AmdVi::empty(),intel:IntelVtd::empty(),stress_passed:0,stress_failed:0,initialized:false}}}
static RUNTIME:SpinLock<BackendRuntime>=SpinLock::new(BackendRuntime::new());
fn with<R>(f:impl FnOnce(&mut BackendRuntime)->R)->R{let mut runtime=RUNTIME.lock();f(&mut runtime)}
pub fn initialize(c:&AcpiCatalog)->Result<(),&'static str>{with(|r|{if r.initialized{return Err("K11 backends already initialized")}
 match AmdVi::discover(c){Ok(mut a)=>{a.enable_units()?;r.amd=a;r.active=ActiveIommu::AmdVi;serial::println(format_args!("[IOMMU] AMD-Vi discovered: {} unit(s), default-deny active",r.amd.count));},Err(_)=>match IntelVtd::discover(c){Ok(mut v)=>{v.enable_units()?;r.intel=v;r.active=ActiveIommu::IntelVtd;serial::println(format_args!("[IOMMU] Intel VT-d discovered: {} unit(s), default-deny active",r.intel.count));},Err(_)=>serial::println(format_args!("[IOMMU] No IVRS/DMAR backend; external DMA remains unauthorized")),}}
 let report=k11_stress::run();r.stress_passed=report.passed;r.stress_failed=report.failed;r.initialized=true;if report.failed!=0{return Err("K11 backend stress self-test failed")}serial::println(format_args!("[TEST] K11.1-K11.8 backend self-tests passed={}",report.passed));Ok(())})}
pub fn active_iommu()->ActiveIommu{with(|r|r.active)}

/// Physical MMIO base of the first Intel VT-d remapping unit discovered from
/// DMAR. K14.B uses this only after the retained K11 discovery gate has passed.
pub fn intel_primary_register_base()->Option<u64>{with(|r|{
 if r.active!=ActiveIommu::IntelVtd||r.intel.count==0{return None}
 for unit in &r.intel.units[..r.intel.count]{if unit.segment==0&&unit.include_all{return Some(unit.register_base)}}
 for unit in &r.intel.units[..r.intel.count]{if unit.segment==0{return Some(unit.register_base)}}
 None
})}

/// Physical MMIO base of the first AMD-Vi unit discovered from IVRS.  The
/// K14.B QEMU qualification uses Intel VT-d; this accessor reserves the same
/// backend-neutral handoff for the later bare-metal AMD qualification path.
pub fn amd_primary_register_base()->Option<u64>{with(|r|{
 if r.active!=ActiveIommu::AmdVi||r.amd.count==0{return None}
 Some(r.amd.units[0].mmio)
})}
