//! K14.C1 native GPU binding foundation.
//!
//! K14.B proved the IOMMU engine with a short-lived EDU domain.  K14.C1 moves
//! native vendor support one step forward by selecting a concrete GPU, claiming
//! it through ForgeBus, inventorying BARs without destructive sizing, and
//! creating the backend-neutral VRAM/GTT ownership model.  It deliberately does
//! not claim that the GPU has a persistent hardware-translated domain yet.

use crate::{
    amd_gpu,
    gpu_memory::{GpuMemoryManager, MemoryDomain},
    gpu_topology,
    native_gpu::{self, NativeGpuVendor, NativeIommuReadiness},
    pci::{self, PciFunction},
    serial,
    sync::SpinLock,
};

pub const NATIVE_BINDING_ABI_VERSION: u32 = 1;
const QUAL_SYSTEM_BUDGET: u64 = 64 << 20;
const QUAL_GTT_BUDGET: u64 = 256 << 20;
const QUAL_VRAM_BUDGET: u64 = 512 << 20;

#[derive(Clone, Copy, Debug)]
pub struct NativeBindingState {
    pub candidates: u32,
    pub selected_vendor: u8,
    pub selected_bus: u8,
    pub selected_device: u8,
    pub selected_function: u8,
    pub forge_claimed: bool,
    pub software_dma_domain: bool,
    pub translation_engine_qualified: bool,
    pub persistent_device_domain: bool,
    pub bar_inventory_ready: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
}

impl NativeBindingState {
    pub const EMPTY: Self = Self {
        candidates: 0,
        selected_vendor: 0,
        selected_bus: 0,
        selected_device: 0,
        selected_function: 0,
        forge_claimed: false,
        software_dma_domain: false,
        translation_engine_qualified: false,
        persistent_device_domain: false,
        bar_inventory_ready: false,
        bus_master_enabled: false,
        fallback_armed: true,
    };
}

static STATE: SpinLock<NativeBindingState> = SpinLock::new(NativeBindingState::EMPTY);

#[must_use]
pub fn state() -> NativeBindingState { *STATE.lock() }

fn vendor_rank(function: PciFunction) -> u8 {
    match function.vendor_id {
        gpu_topology::VENDOR_AMD => 0,
        gpu_topology::VENDOR_INTEL => 1,
        gpu_topology::VENDOR_NVIDIA => 2,
        _ => 255,
    }
}

fn select_native_candidate() -> (u32, Option<PciFunction>) {
    let mut candidates = 0u32;
    let mut selected: Option<PciFunction> = None;
    pci::enumerate(|function| {
        if function.class_code != gpu_topology::PCI_CLASS_DISPLAY
            || native_gpu::profile(function.vendor_id).is_none()
        {
            return;
        }
        candidates = candidates.saturating_add(1);
        match selected {
            None => selected = Some(function),
            Some(current) if vendor_rank(function) < vendor_rank(current) => selected = Some(function),
            _ => {}
        }
    });
    (candidates, selected)
}

fn memory_ownership_self_test() -> Result<(usize, u64, u64), &'static str> {
    let mut memory = GpuMemoryManager::new(QUAL_SYSTEM_BUDGET, QUAL_GTT_BUDGET, QUAL_VRAM_BUDGET);
    let owner = 0x14c1u64;
    let scanout = memory.create(owner, 16 << 20, 4096, MemoryDomain::Vram)?;
    let staging = memory.create(owner, 8 << 20, 4096, MemoryDomain::System)?;
    memory.migrate(owner, staging, MemoryDomain::Gtt)?;
    memory.pin(owner, scanout, true)?;
    if memory.destroy(owner, scanout).is_ok() {
        return Err("native GPU pinned VRAM object was destructible");
    }
    Ok((memory.active_count(), memory.used_bytes(MemoryDomain::Vram), memory.used_bytes(MemoryDomain::Gtt)))
}

pub fn initialize_binding_foundation() -> Result<NativeBindingState, &'static str> {
    amd_gpu::self_test()?;
    serial::println(format_args!(
        "[AMDB] AMD backend foundation self-test: abi={} firmware_components={} vendor_mmio_writes=false",
        amd_gpu::AMD_NATIVE_BACKEND_ABI_VERSION,
        amd_gpu::AMD_FIRMWARE_COMPONENTS
    ));

    let (objects, vram, gtt) = memory_ownership_self_test()?;
    serial::println(format_args!(
        "[NVRM] native VRAM/GTT ownership self-test: objects={} vram_bytes={} gtt_bytes={} pinned_guard=true",
        objects, vram, gtt
    ));

    let (candidates, selected) = select_native_candidate();
    let readiness = native_gpu::current_iommu_readiness();
    let translation_engine_qualified = readiness == NativeIommuReadiness::HardwareTranslated;
    let mut report = NativeBindingState {
        candidates,
        translation_engine_qualified,
        ..NativeBindingState::EMPTY
    };

    match selected {
        None => {
            serial::println(format_args!(
                "[NSEL] K14.C1 native backend selection: candidates=0 selected=none qemu_fallback=true"
            ));
            serial::println(format_args!(
                "[NBND] native ForgeBus ownership: selected=none claimed=false software_dma_domain=false bus_master=false"
            ));
        }
        Some(function) => {
            let Some(profile) = native_gpu::profile(function.vendor_id) else {
                return Err("selected native GPU lacks vendor profile");
            };
            report.selected_vendor = profile.vendor as u8;
            report.selected_bus = function.bus;
            report.selected_device = function.device;
            report.selected_function = function.function;
            serial::println(format_args!(
                "[NSEL] K14.C1 native backend selection: candidates={} selected={} at {:02x}:{:02x}.{} vendor={:#06x} device={:#06x}",
                candidates, profile.name, function.bus, function.device, function.function,
                function.vendor_id, function.device_id
            ));

            if profile.vendor == NativeGpuVendor::Amd {
                let binding = amd_gpu::claim_foundation(function)?;
                report.forge_claimed = true;
                report.software_dma_domain = binding.software_dma_domain;
                report.bar_inventory_ready = binding.bars.present != 0;
                report.bus_master_enabled = binding.bus_master_enabled;
                serial::println(format_args!(
                    "[NBND] native ForgeBus ownership: selected=AMD device={} claimed=true software_dma_domain={} bars={} bus_master=false",
                    binding.device.0, binding.software_dma_domain, binding.bars.present
                ));
            } else {
                // Intel/NVIDIA are recognized now but stay probe-only until their
                // vendor backend is implemented behind the same contract.
                serial::println(format_args!(
                    "[NBND] native ForgeBus ownership: selected={} claimed=false software_dma_domain=false vendor_backend_deferred=true bus_master=false",
                    profile.name
                ));
            }
        }
    }

    // K14.B's qualification domain is intentionally torn down.  Therefore a
    // physical GPU cannot be marked translated merely because the engine was
    // proven.  K14.C2 must install and retain a domain for the selected RID.
    report.persistent_device_domain = false;
    report.bus_master_enabled = false;
    report.fallback_armed = true;
    serial::println(format_args!(
        "[NDOM] native translated-domain admission: engine_qualified={} persistent_device_domain=false bus_master=false",
        report.translation_engine_qualified
    ));
    serial::println(format_args!(
        "[NCF ] K14.C1 native binding foundation ready: candidates={} selected_vendor={} claimed={} persistent_domain={} activation=false fallback_armed=true",
        report.candidates, report.selected_vendor, report.forge_claimed, report.persistent_device_domain
    ));
    *STATE.lock() = report;
    Ok(report)
}

/// Packed K14.C1 status for DISPLAYD.
/// bits 0..7 candidate count, bits 8..15 selected vendor,
/// bit 16 ForgeBus claimed, bit 17 software DMA domain,
/// bit 18 translation engine qualified, bit 19 persistent device domain,
/// bit 20 BAR inventory ready, bit 21 bus master enabled, bit 22 fallback armed.
#[must_use]
pub fn packed_status() -> u64 {
    let state = state();
    let mut value = u64::from(state.candidates & 0xff) | (u64::from(state.selected_vendor) << 8);
    if state.forge_claimed { value |= 1 << 16; }
    if state.software_dma_domain { value |= 1 << 17; }
    if state.translation_engine_qualified { value |= 1 << 18; }
    if state.persistent_device_domain { value |= 1 << 19; }
    if state.bar_inventory_ready { value |= 1 << 20; }
    if state.bus_master_enabled { value |= 1 << 21; }
    if state.fallback_armed { value |= 1 << 22; }
    value
}
