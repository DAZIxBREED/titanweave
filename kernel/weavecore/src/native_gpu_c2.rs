//! K14.C2 persistent translated-domain and AMD bring-up contract.
//!
//! QEMU cannot emulate a native Radeon. C2 therefore proves that one requester
//! can retain a real VT-d translated domain across multiple DMA epochs, while
//! separately advancing the AMD backend's ASIC/firmware/ring bring-up contract.
//! Physical AMD MMIO writes, firmware upload, GPU bus mastering, and command
//! submission remain fail-closed until a domain is bound to the actual Radeon RID.

use crate::{
    amd_gpu,
    memory::FrameAllocator,
    native_gpu::{NativeDriverPhase, NativeGpuVendor},
    native_gpu_binding,
    serial,
    sync::SpinLock,
    translated_dma,
};

pub const K14C2_ABI_VERSION: u32 = 1;
pub const AMD_REQUIRED_FIRMWARE_MASK: u32 = 0x3f; // VBIOS, PSP, SMU, GC, SDMA, scheduler/MES family.
pub const AMD_BOOTSTRAP_RING_ENTRIES: u32 = 1024;
pub const AMD_BOOTSTRAP_RING_ALIGNMENT: u64 = 4096;

#[derive(Clone, Copy, Debug)]
pub struct C2State {
    pub surrogate_domain_qualified: bool,
    pub persistent_epochs: u32,
    pub amd_candidate: bool,
    pub firmware_plan_ready: bool,
    pub ring_plan_ready: bool,
    pub actual_gpu_domain_bound: bool,
    pub vendor_mmio_writes: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
}
impl C2State {
    pub const EMPTY: Self = Self { surrogate_domain_qualified:false,persistent_epochs:0,amd_candidate:false,
        firmware_plan_ready:false,ring_plan_ready:false,actual_gpu_domain_bound:false,vendor_mmio_writes:false,
        bus_master_enabled:false,fallback_armed:true };
}
static STATE: SpinLock<C2State> = SpinLock::new(C2State::EMPTY);

fn self_test() -> Result<(), &'static str> {
    if AMD_REQUIRED_FIRMWARE_MASK.count_ones() != 6 { return Err("AMD firmware plan mask is incomplete"); }
    if !AMD_BOOTSTRAP_RING_ENTRIES.is_power_of_two() || AMD_BOOTSTRAP_RING_ALIGNMENT < 4096 {
        return Err("AMD command-ring bootstrap geometry is invalid");
    }
    if !NativeDriverPhase::Claimed.may_transition_to(NativeDriverPhase::MmioMapped)
        || !NativeDriverPhase::FirmwareReady.may_transition_to(NativeDriverPhase::DmaTranslated)
        || !NativeDriverPhase::DmaTranslated.may_transition_to(NativeDriverPhase::QueuesReady)
    { return Err("AMD C2 lifecycle ordering failed"); }
    Ok(())
}

pub fn initialize(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<C2State, &'static str> {
    self_test()?;
    let persistent = translated_dma::qualify_persistent_domain_surrogate(allocator, kernel_cr3)?;
    let binding = native_gpu_binding::state();
    let amd_candidate = binding.selected_vendor == NativeGpuVendor::Amd as u8;
    let firmware_plan_ready = amd_gpu::AMD_FIRMWARE_COMPONENTS >= 4 && AMD_REQUIRED_FIRMWARE_MASK != 0;
    let ring_plan_ready = AMD_BOOTSTRAP_RING_ENTRIES.is_power_of_two();

    serial::println(format_args!(
        "[AFWP] K14.C2 AMD firmware bring-up plan: mask={:#04x} vbios=true psp=true smu=true gc=true sdma=true scheduler=true upload=false",
        AMD_REQUIRED_FIRMWARE_MASK
    ));
    serial::println(format_args!(
        "[ARING] K14.C2 AMD ring plan: entries={} alignment={} doorbell_page=4096 submission=false",
        AMD_BOOTSTRAP_RING_ENTRIES, AMD_BOOTSTRAP_RING_ALIGNMENT
    ));
    if amd_candidate {
        serial::println(format_args!(
            "[ASIC] K14.C2 AMD candidate identity: {:02x}:{:02x}.{} claimed={} bar_inventory={} mmio_writes=false",
            binding.selected_bus, binding.selected_device, binding.selected_function,
            binding.forge_claimed, binding.bar_inventory_ready
        ));
    } else {
        serial::println(format_args!(
            "[ASIC] K14.C2 AMD candidate identity: none qemu_surrogate_only=true mmio_writes=false"
        ));
    }

    // The QEMU surrogate proves persistence mechanics, not a Radeon domain.
    // Never promote the actual GPU gate from a surrogate requester.
    let actual_gpu_domain_bound = false;
    let state = C2State {
        surrogate_domain_qualified: persistent.hardware_translated && persistent.epochs >= 3 && persistent.revoked,
        persistent_epochs: persistent.epochs,
        amd_candidate,
        firmware_plan_ready,
        ring_plan_ready,
        actual_gpu_domain_bound,
        vendor_mmio_writes: false,
        bus_master_enabled: false,
        fallback_armed: true,
    };
    serial::println(format_args!(
        "[C2RD] K14.C2 native bring-up contract ready: surrogate_domain={} epochs={} amd_candidate={} firmware_plan={} ring_plan={} actual_gpu_domain=false bus_master=false fallback=true",
        state.surrogate_domain_qualified, state.persistent_epochs, state.amd_candidate,
        state.firmware_plan_ready, state.ring_plan_ready
    ));
    *STATE.lock() = state;
    Ok(state)
}

#[must_use]
pub fn state() -> C2State { *STATE.lock() }

/// Packed status for DISPLAYD. bit0 surrogate domain proof, bit1 AMD candidate,
/// bit2 firmware plan, bit3 ring plan, bit4 actual GPU domain, bit5 bus master,
/// bit6 fallback; epochs are stored in bits 8..15.
#[must_use]
pub fn packed_status() -> u64 {
    let s=state(); let mut v=(u64::from(s.persistent_epochs & 0xff))<<8;
    if s.surrogate_domain_qualified {v|=1<<0;} if s.amd_candidate {v|=1<<1;}
    if s.firmware_plan_ready {v|=1<<2;} if s.ring_plan_ready {v|=1<<3;}
    if s.actual_gpu_domain_bound {v|=1<<4;} if s.bus_master_enabled {v|=1<<5;}
    if s.fallback_armed {v|=1<<6;} v
}
