//! K14.A native GPU prerequisite and K14.B translated-DMA safety foundation.
//!
//! K13 qualified a backend-neutral GPU model and a live VirtIO-GPU transport.
//! K14 begins native AMD/Intel/NVIDIA support.  This first slice is deliberately
//! read-only: PCI identity/configuration and BAR inventory may be inspected, but
//! discovery does not claim the device, map vendor MMIO, enable memory decoding,
//! enable bus mastering, upload firmware, or create DMA mappings.
//!
//! Native activation is gated on real hardware-translated IOMMU mappings.  K11's
//! current AMD-Vi/VT-d backends provide discovery/default-deny policy scaffolding,
//! but do not yet install hardware translation page tables, so K14.A reports
//! PolicyOnly and retains the qualified K13 VirtIO + K12 GOP fallback paths.

use crate::{gpu_topology, k11_backends, pci, serial, sync::SpinLock, translated_dma};

pub const NATIVE_GPU_DRIVER_ABI_VERSION: u32 = 1;
pub const MAX_NATIVE_GPU_ADAPTERS: usize = 8;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeGpuVendor {
    Amd = 1,
    Intel = 2,
    Nvidia = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeDriverPhase {
    Discovered = 1,
    Quarantined = 2,
    Claimed = 3,
    MmioMapped = 4,
    FirmwareReady = 5,
    DmaTranslated = 6,
    QueuesReady = 7,
    DisplayReady = 8,
    AccelerationReady = 9,
    Failed = 255,
}

impl NativeDriverPhase {
    #[must_use]
    pub const fn may_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Discovered, Self::Quarantined)
                | (Self::Quarantined, Self::Claimed)
                | (Self::Claimed, Self::MmioMapped)
                | (Self::MmioMapped, Self::FirmwareReady)
                | (Self::FirmwareReady, Self::DmaTranslated)
                | (Self::DmaTranslated, Self::QueuesReady)
                | (Self::QueuesReady, Self::DisplayReady)
                | (Self::DisplayReady, Self::AccelerationReady)
                | (_, Self::Failed)
        )
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeIommuReadiness {
    Unavailable = 0,
    PolicyOnly = 1,
    HardwareTranslated = 2,
}

#[derive(Clone, Copy, Debug)]
pub struct NativeGpuProfile {
    pub vendor: NativeGpuVendor,
    pub name: &'static str,
    pub discrete: bool,
    pub minimum_dma_bits: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct NativeGpuProbeReport {
    pub adapters: u32,
    pub amd: u32,
    pub intel: u32,
    pub nvidia: u32,
    pub mmio_bars: u32,
    pub memory_decode_enabled: u32,
    pub preexisting_bus_master: u32,
    pub first: Option<pci::PciFunction>,
}

#[derive(Clone, Copy, Debug)]
pub struct NativeGpuState {
    pub adapters: u32,
    pub amd: u32,
    pub intel: u32,
    pub nvidia: u32,
    pub preexisting_bus_master: u32,
    pub iommu_readiness: NativeIommuReadiness,
    pub activation_ready: bool,
}

impl NativeGpuState {
    pub const EMPTY: Self = Self {
        adapters: 0,
        amd: 0,
        intel: 0,
        nvidia: 0,
        preexisting_bus_master: 0,
        iommu_readiness: NativeIommuReadiness::Unavailable,
        activation_ready: false,
    };
}

static STATE: SpinLock<NativeGpuState> = SpinLock::new(NativeGpuState::EMPTY);

/// Vendor-neutral contract all later K14 native backends must implement.
///
/// The ordering is intentional: translated DMA is established before queues or
/// acceleration.  Backend implementations may fail closed at any step and the
/// compositor must retain the K13/K12 fallback path.
pub trait NativeGpuBackend {
    fn vendor(&self) -> NativeGpuVendor;
    fn phase(&self) -> NativeDriverPhase;
    fn claim(&mut self, function: pci::PciFunction) -> Result<(), &'static str>;
    fn map_register_apertures(&mut self) -> Result<(), &'static str>;
    fn prepare_firmware(&mut self) -> Result<(), &'static str>;
    fn bind_translated_dma(&mut self) -> Result<(), &'static str>;
    fn start_queues(&mut self) -> Result<(), &'static str>;
    fn start_display(&mut self) -> Result<(), &'static str>;
    fn start_acceleration(&mut self) -> Result<(), &'static str>;
    fn fence_and_reset(&mut self) -> Result<(), &'static str>;
}

#[must_use]
pub const fn profile(vendor_id: u16) -> Option<NativeGpuProfile> {
    match vendor_id {
        gpu_topology::VENDOR_AMD => Some(NativeGpuProfile {
            vendor: NativeGpuVendor::Amd,
            name: "AMD",
            discrete: true,
            minimum_dma_bits: 40,
        }),
        gpu_topology::VENDOR_INTEL => Some(NativeGpuProfile {
            vendor: NativeGpuVendor::Intel,
            name: "Intel",
            discrete: false,
            minimum_dma_bits: 39,
        }),
        gpu_topology::VENDOR_NVIDIA => Some(NativeGpuProfile {
            vendor: NativeGpuVendor::Nvidia,
            name: "NVIDIA",
            discrete: true,
            minimum_dma_bits: 40,
        }),
        _ => None,
    }
}

fn memory_bar_count(function: pci::PciFunction) -> u32 {
    let count = if function.header_type & 0x7f == 0 { 6u8 } else { 2u8 };
    let mut index = 0u8;
    let mut bars = 0u32;
    while index < count {
        let offset = 0x10u8 + index * 4;
        let raw = pci::read_u32(function.bus, function.device, function.function, offset);
        if raw != 0 && raw != 0xffff_ffff && raw & 1 == 0 {
            bars = bars.saturating_add(1);
            if ((raw >> 1) & 3) == 2 && index + 1 < count {
                index += 2;
                continue;
            }
        }
        index += 1;
    }
    bars
}

/// Side-effect-free native adapter discovery.
///
/// This routine only reads PCI configuration space.  Keep it that way: K14.A
/// qualification explicitly rejects writes, ForgeBus claims, MMIO mappings and
/// bus-master enabling from this discovery path.
#[must_use]
pub fn discover_probe_only() -> NativeGpuProbeReport {
    let mut report = NativeGpuProbeReport {
        adapters: 0,
        amd: 0,
        intel: 0,
        nvidia: 0,
        mmio_bars: 0,
        memory_decode_enabled: 0,
        preexisting_bus_master: 0,
        first: None,
    };
    pci::enumerate(|function| {
        if function.class_code != gpu_topology::PCI_CLASS_DISPLAY {
            return;
        }
        let Some(profile) = profile(function.vendor_id) else { return; };
        report.adapters = report.adapters.saturating_add(1);
        match profile.vendor {
            NativeGpuVendor::Amd => report.amd = report.amd.saturating_add(1),
            NativeGpuVendor::Intel => report.intel = report.intel.saturating_add(1),
            NativeGpuVendor::Nvidia => report.nvidia = report.nvidia.saturating_add(1),
        }
        report.mmio_bars = report.mmio_bars.saturating_add(memory_bar_count(function));
        let command = pci::read_u16(function.bus, function.device, function.function, 0x04);
        if command & (1 << 1) != 0 { report.memory_decode_enabled = report.memory_decode_enabled.saturating_add(1); }
        if command & (1 << 2) != 0 { report.preexisting_bus_master = report.preexisting_bus_master.saturating_add(1); }
        if report.first.is_none() && (report.adapters as usize) <= MAX_NATIVE_GPU_ADAPTERS {
            report.first = Some(function);
        }
    });
    report
}

/// K14.B promotes the IOMMU readiness level only after a live translated-DMA
/// qualification has crossed the hardware engine.  Merely discovering DMAR or
/// IVRS still counts as PolicyOnly.
#[must_use]
pub fn current_iommu_readiness() -> NativeIommuReadiness {
    if translated_dma::hardware_translation_qualified() {
        return NativeIommuReadiness::HardwareTranslated;
    }
    match k11_backends::active_iommu() {
        k11_backends::ActiveIommu::None => NativeIommuReadiness::Unavailable,
        k11_backends::ActiveIommu::AmdVi | k11_backends::ActiveIommu::IntelVtd => NativeIommuReadiness::PolicyOnly,
    }
}

pub fn authorize_bus_mastering(
    claimed: bool,
    readiness: NativeIommuReadiness,
) -> Result<(), &'static str> {
    if !claimed { return Err("native GPU is not owned by ForgeBus"); }
    if readiness != NativeIommuReadiness::HardwareTranslated {
        return Err("native GPU DMA requires hardware-translated IOMMU mappings");
    }
    Ok(())
}

fn run_contract_self_test() -> Result<(), &'static str> {
    for vendor in [gpu_topology::VENDOR_AMD, gpu_topology::VENDOR_INTEL, gpu_topology::VENDOR_NVIDIA] {
        if profile(vendor).is_none() { return Err("native GPU vendor profile missing"); }
    }
    if !NativeDriverPhase::Discovered.may_transition_to(NativeDriverPhase::Quarantined)
        || NativeDriverPhase::Discovered.may_transition_to(NativeDriverPhase::AccelerationReady)
        || !NativeDriverPhase::DisplayReady.may_transition_to(NativeDriverPhase::AccelerationReady)
    {
        return Err("native GPU lifecycle transition policy failed");
    }
    if authorize_bus_mastering(true, NativeIommuReadiness::PolicyOnly).is_ok()
        || authorize_bus_mastering(false, NativeIommuReadiness::HardwareTranslated).is_ok()
        || authorize_bus_mastering(true, NativeIommuReadiness::HardwareTranslated).is_err()
    {
        return Err("native GPU DMA admission gate failed");
    }
    Ok(())
}

pub fn initialize_foundation() -> Result<NativeGpuState, &'static str> {
    run_contract_self_test()?;
    serial::println(format_args!(
        "[NDRV] native backend contract self-test: abi={} vendors=3 lifecycle=guarded",
        NATIVE_GPU_DRIVER_ABI_VERSION
    ));

    let probe = discover_probe_only();
    serial::println(format_args!(
        "[NGPU] native adapter probe: adapters={} amd={} intel={} nvidia={} mmio_bars={} memory_decode={} preexisting_bus_master={} probe_only=true",
        probe.adapters, probe.amd, probe.intel, probe.nvidia, probe.mmio_bars,
        probe.memory_decode_enabled, probe.preexisting_bus_master
    ));
    if let Some(first) = probe.first {
        let command = pci::read_u16(first.bus, first.device, first.function, 0x04);
        serial::println(format_args!(
            "[NBAR] first native candidate {:02x}:{:02x}.{} vendor={:#06x} device={:#06x} command={:#06x} read_only=true",
            first.bus, first.device, first.function, first.vendor_id, first.device_id, command
        ));
    }

    let iommu_readiness = current_iommu_readiness();
    // K14.B proves the translation engine, but deliberately does not claim a
    // physical AMD/Intel/NVIDIA adapter or leave a native device domain active.
    // K14.C must bind that per-device domain before bus mastering is admitted.
    let native_domain_bound = false;
    let activation_ready = probe.adapters != 0
        && native_domain_bound
        && authorize_bus_mastering(true, iommu_readiness).is_ok();
    serial::println(format_args!(
        "[IOMQ] native DMA admission: iommu={:?} hardware_translation={} device_domain_bound={} bus_master_authorized={}",
        iommu_readiness,
        iommu_readiness == NativeIommuReadiness::HardwareTranslated,
        native_domain_bound,
        activation_ready
    ));
    if !activation_ready {
        serial::println(format_args!(
            "[NFAL] native activation deferred; K13 VirtIO-GPU and K12 GOP fallback remain armed"
        ));
    }

    let state = NativeGpuState {
        adapters: probe.adapters,
        amd: probe.amd,
        intel: probe.intel,
        nvidia: probe.nvidia,
        preexisting_bus_master: probe.preexisting_bus_master,
        iommu_readiness,
        activation_ready,
    };
    *STATE.lock() = state;
    Ok(state)
}

#[must_use]
pub fn state() -> NativeGpuState { *STATE.lock() }

/// Packed K14 native GPU status for DISPLAYD.
/// bits 0..7 adapters, 8..15 AMD, 16..23 Intel, 24..31 NVIDIA,
/// bit 32 IOMMU policy present, bit 33 hardware translation ready,
/// bit 34 firmware/preexisting bus master observed, bit 35 native activation ready.
#[must_use]
pub fn packed_status() -> u64 {
    let state = state();
    let mut value = u64::from(state.adapters & 0xff)
        | (u64::from(state.amd & 0xff) << 8)
        | (u64::from(state.intel & 0xff) << 16)
        | (u64::from(state.nvidia & 0xff) << 24);
    if state.iommu_readiness != NativeIommuReadiness::Unavailable { value |= 1 << 32; }
    if state.iommu_readiness == NativeIommuReadiness::HardwareTranslated { value |= 1 << 33; }
    if state.preexisting_bus_master != 0 { value |= 1 << 34; }
    if state.activation_ready { value |= 1 << 35; }
    value
}
