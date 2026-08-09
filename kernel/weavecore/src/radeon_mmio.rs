//! K14.C27 permanent Radeon reviewed-MMIO service.
//!
//! C21-C26 proved two exact GFX12 scratch targets.  This module converts those
//! milestone-local proofs into a reusable driver-owned API.  It intentionally
//! accepts register *identities*, never caller-supplied addresses.  Generic
//! writes are rejected; the only write authority retained by K14 is the frozen
//! C22-C25 reversible SCRATCH_REG0 qualification procedure.

use crate::{
    memory::FrameAllocator,
    native_gpu_binding,
    native_gpu_c19,
    native_gpu_c21,
    native_gpu_c26,
    paging,
    pci,
    sync::SpinLock,
};

pub const RADEON_MMIO_SERVICE_ABI_VERSION: u32 = 1;
pub const RADEON_MMIO_REVIEWED_TARGETS: u8 = 2;
pub const RADEON_MMIO_GENERIC_WRITE_ALLOWED: bool = false;
pub const RADEON_MMIO_CALLER_ADDRESS_ALLOWED: bool = false;
pub const RADEON_MMIO_CALLER_VALUE_ALLOWED: bool = false;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewedRegister { ScratchReg0 = 0, ScratchReg1 = 1 }

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessClass { FrozenReversibleProbe = 1, ReadOnly = 2 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewedTarget { pub register: ReviewedRegister, pub access: AccessClass, pub dword: u32, pub base_idx: u8, pub target_dword_offset: u64, pub target_byte_offset: u64 }

#[derive(Clone, Copy, Debug)]
pub struct MmioServiceState {
    pub policy_ready: bool, pub amd_present: bool, pub hardware_deferred: bool,
    pub bar5_ready: bool, pub memory_decode_on: bool, pub bus_master_off: bool,
    pub reg0_resolved: bool, pub reg1_resolved: bool, pub reg0_read_valid: bool,
    pub reg1_read_valid: bool, pub reg0_value: u32, pub reg1_value: u32,
    pub reads_performed: u8, pub writes_performed: u8, pub generic_write_rejected: bool,
    pub fingerprint: u64,
}
impl MmioServiceState { pub const EMPTY: Self = Self { policy_ready:false, amd_present:false, hardware_deferred:true, bar5_ready:false, memory_decode_on:false, bus_master_off:false, reg0_resolved:false, reg1_resolved:false, reg0_read_valid:false, reg1_read_valid:false, reg0_value:0, reg1_value:0, reads_performed:0, writes_performed:0, generic_write_rejected:false, fingerprint:0 }; }
static STATE: SpinLock<MmioServiceState> = SpinLock::new(MmioServiceState::EMPTY);

fn selected_function() -> Option<pci::PciFunction> {
    let b = native_gpu_binding::state(); let mut found = None;
    pci::enumerate(|f| { if f.bus == b.selected_bus && f.device == b.selected_device && f.function == b.selected_function { found = Some(f); } }); found
}

pub fn target(register: ReviewedRegister) -> Result<ReviewedTarget, &'static str> {
    let c26 = native_gpu_c26::state();
    match register {
        ReviewedRegister::ScratchReg0 => {
            if c26.amd_present && (!c26.c25_target_revalidated || c26.reg0_target_byte_offset == 0) { return Err("Radeon MMIO REG0 target lacks frozen C25/C26 proof"); }
            Ok(ReviewedTarget { register, access: AccessClass::FrozenReversibleProbe, dword: native_gpu_c21::GFX12_SCRATCH_REG0_DWORD, base_idx: native_gpu_c21::GFX12_SCRATCH_REG0_BASE_IDX, target_dword_offset: c26.reg0_target_dword_offset, target_byte_offset: c26.reg0_target_byte_offset })
        }
        ReviewedRegister::ScratchReg1 => {
            if c26.amd_present && (!c26.reg1_resolved || c26.reg1_target_byte_offset == 0) { return Err("Radeon MMIO REG1 target lacks frozen C26 proof"); }
            Ok(ReviewedTarget { register, access: AccessClass::ReadOnly, dword: native_gpu_c26::GFX12_SCRATCH_REG1_DWORD, base_idx: native_gpu_c26::GFX12_SCRATCH_REG1_BASE_IDX, target_dword_offset: c26.reg1_target_dword_offset, target_byte_offset: c26.reg1_target_byte_offset })
        }
    }
}

pub fn authorize_generic_write(register: ReviewedRegister) -> Result<(), &'static str> { let _ = target(register)?; Err("generic Radeon MMIO writes are forbidden; use a milestone-reviewed transaction") }

pub fn read_reviewed(allocator: &mut FrameAllocator<'_>, kernel_cr3: u64, register: ReviewedRegister) -> Result<u32, &'static str> {
    let c26 = native_gpu_c26::state(); if !c26.amd_present { return Err("no physical Radeon for reviewed MMIO read"); }
    let target = target(register)?; let function = selected_function().ok_or("selected Radeon PCI function disappeared")?;
    let command = pci::read_u16(function.bus, function.device, function.function, 0x04);
    if command & (1 << 1) == 0 { return Err("Radeon PCI memory decode is disabled"); }
    if command & (1 << 2) != 0 { return Err("Radeon bus mastering must remain disabled in C27"); }
    let bar5 = pci::memory_bar_base(function, native_gpu_c26::RADEON_C26_MMIO_BAR_INDEX).ok_or("Radeon BAR5 is unavailable")?;
    let page_bytes = native_gpu_c26::RADEON_C26_PAGE_BYTES; let page_off = target.target_byte_offset & !(page_bytes - 1); let in_page = target.target_byte_offset & (page_bytes - 1);
    if in_page & 3 != 0 || in_page + 4 > page_bytes { return Err("reviewed Radeon target crosses or misaligns an MMIO page"); }
    let phys = bar5.checked_add(page_off).ok_or("Radeon MMIO physical address overflow")?;
    let virt = paging::map_kernel_mmio_readonly(allocator, kernel_cr3, phys, page_bytes)?;
    let value = unsafe { core::ptr::read_volatile((virt + in_page) as *const u32) };
    if value == u32::MAX { return Err("reviewed Radeon MMIO read returned open-bus all-ones"); }
    Ok(value)
}

fn mix(mut h:u64, value:u64)->u64 { h ^= value; h = h.wrapping_mul(0x100000001b3); h }
fn self_test() -> Result<(), &'static str> {
    if RADEON_MMIO_SERVICE_ABI_VERSION != 1 || RADEON_MMIO_REVIEWED_TARGETS != 2 || RADEON_MMIO_GENERIC_WRITE_ALLOWED || RADEON_MMIO_CALLER_ADDRESS_ALLOWED || RADEON_MMIO_CALLER_VALUE_ALLOWED { return Err("Radeon MMIO service constants violate C27 policy"); }
    if authorize_generic_write(ReviewedRegister::ScratchReg0).is_ok() || authorize_generic_write(ReviewedRegister::ScratchReg1).is_ok() { return Err("Radeon MMIO generic write policy self-test failed"); }
    let a = target(ReviewedRegister::ScratchReg0)?; let b = target(ReviewedRegister::ScratchReg1)?;
    if a.dword + 1 != b.dword || a.base_idx != b.base_idx || a.access != AccessClass::FrozenReversibleProbe || b.access != AccessClass::ReadOnly { return Err("Radeon MMIO reviewed target policy self-test failed"); }
    Ok(())
}

pub fn initialize(allocator: &mut FrameAllocator<'_>, kernel_cr3: u64) -> Result<MmioServiceState, &'static str> {
    self_test()?; let c26 = native_gpu_c26::state();
    let mut s = MmioServiceState { policy_ready:true, amd_present:c26.amd_present, hardware_deferred:!c26.amd_present, generic_write_rejected:authorize_generic_write(ReviewedRegister::ScratchReg0).is_err() && authorize_generic_write(ReviewedRegister::ScratchReg1).is_err(), ..MmioServiceState::EMPTY };
    if !c26.amd_present { s.fingerprint = mix(0xc27a_0000_0000_0001, u64::from(RADEON_MMIO_REVIEWED_TARGETS)); *STATE.lock() = s; return Ok(s); }
    if !c26.k14_completion_verified || !c26.allowlist_exact || !c26.no_write_verified { return Err("Radeon MMIO service requires frozen C26 allowlist proof"); }
    if native_gpu_c19::state().snapshot_fingerprint != c26.snapshot_fingerprint { return Err("Radeon MMIO service snapshot lineage changed after C26"); }
    let function = selected_function().ok_or("selected Radeon disappeared during MMIO service init")?; let command = pci::read_u16(function.bus,function.device,function.function,0x04);
    s.memory_decode_on = command & (1<<1) != 0; s.bus_master_off = command & (1<<2) == 0; s.bar5_ready = pci::memory_bar_base(function,native_gpu_c26::RADEON_C26_MMIO_BAR_INDEX).is_some();
    if !s.memory_decode_on || !s.bus_master_off || !s.bar5_ready { return Err("Radeon MMIO service PCI safety prerequisites failed"); }
    let r0 = target(ReviewedRegister::ScratchReg0)?; let r1 = target(ReviewedRegister::ScratchReg1)?;
    s.reg0_resolved = r0.target_byte_offset != 0; s.reg1_resolved = r1.target_byte_offset != 0;
    s.reg0_value = read_reviewed(allocator,kernel_cr3,ReviewedRegister::ScratchReg0)?; s.reads_performed = s.reads_performed.saturating_add(1);
    s.reg1_value = read_reviewed(allocator,kernel_cr3,ReviewedRegister::ScratchReg1)?; s.reads_performed = s.reads_performed.saturating_add(1);
    s.reg0_read_valid = true; s.reg1_read_valid = true; s.hardware_deferred = false;
    let command_after = pci::read_u16(function.bus,function.device,function.function,0x04);
    if command_after & (1<<1) == 0 || command_after & (1<<2) != 0 { return Err("Radeon PCI command changed during C27 MMIO service reads"); }
    let mut fp=0xc27a_4d4d_494f_0001u64; fp=mix(fp,r0.target_byte_offset); fp=mix(fp,r1.target_byte_offset); fp=mix(fp,u64::from(s.reg0_value)); fp=mix(fp,u64::from(s.reg1_value)); s.fingerprint=fp; *STATE.lock()=s; Ok(s)
}
pub fn state()->MmioServiceState { *STATE.lock() }
