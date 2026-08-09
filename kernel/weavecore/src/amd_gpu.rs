//! K14.C1 AMD-first native GPU backend foundation.
//!
//! This slice establishes ownership and identification rules for a future
//! Radeon backend without touching vendor MMIO or starting GPU engines.  A
//! physical AMD display function may be claimed through ForgeBus and placed in
//! a bounded software DMA domain, but bus mastering stays disabled until K14.C2
//! can install a persistent hardware-translated domain for that exact requester.

use crate::{
    device::DeviceId,
    forgebus,
    gpu_topology,
    native_gpu::{NativeDriverPhase, NativeGpuVendor},
    pci::{self, PciFunction},
};

pub const AMD_NATIVE_BACKEND_ABI_VERSION: u32 = 1;
pub const AMD_FIRMWARE_COMPONENTS: u8 = 4; // VBIOS, PSP, SMU, graphics microcode family.

#[derive(Clone, Copy, Debug)]
pub struct AmdBarInventory {
    pub bars: [u64; 6],
    pub present: u8,
}

impl AmdBarInventory {
    pub const EMPTY: Self = Self { bars: [0; 6], present: 0 };
}

#[derive(Clone, Copy, Debug)]
pub struct AmdBinding {
    pub function: PciFunction,
    pub device: DeviceId,
    pub phase: NativeDriverPhase,
    pub bars: AmdBarInventory,
    pub software_dma_domain: bool,
    pub bus_master_enabled: bool,
}

#[must_use]
pub const fn is_amd_display(function: PciFunction) -> bool {
    function.vendor_id == gpu_topology::VENDOR_AMD
        && function.class_code == gpu_topology::PCI_CLASS_DISPLAY
}

#[must_use]
pub fn inventory_bars(function: PciFunction) -> AmdBarInventory {
    let mut report = AmdBarInventory::EMPTY;
    let mut index = 0u8;
    while index < 6 {
        if let Some(base) = pci::memory_bar_base(function, index) {
            report.bars[index as usize] = base;
            report.present = report.present.saturating_add(1);
        }
        let raw = pci::read_u32(function.bus, function.device, function.function, 0x10 + index * 4);
        if raw & 1 == 0 && ((raw >> 1) & 3) == 2 {
            index = index.saturating_add(2);
        } else {
            index = index.saturating_add(1);
        }
    }
    report
}

/// Claims an actual AMD display function without mapping registers or enabling
/// DMA.  Any firmware-left bus-master bit is cleared before ForgeBus ownership
/// is established so K14.C1 starts from a fail-closed state.
pub fn claim_foundation(function: PciFunction) -> Result<AmdBinding, &'static str> {
    if !is_amd_display(function) {
        return Err("AMD backend received a non-AMD display function");
    }
    let command = pci::read_u16(function.bus, function.device, function.function, 0x04);
    if command & (1 << 2) != 0 {
        pci::disable_bus_master(function);
    }
    let (device, _) = forgebus::claim_pci_function(function, b"titan-amdgpu", 2)?;
    forgebus::establish_dma_domain(device, 48, true)?;
    let bars = inventory_bars(function);
    Ok(AmdBinding {
        function,
        device,
        phase: NativeDriverPhase::Claimed,
        bars,
        software_dma_domain: true,
        bus_master_enabled: false,
    })
}

pub fn self_test() -> Result<(), &'static str> {
    let sample = PciFunction {
        bus: 4,
        device: 0,
        function: 0,
        vendor_id: gpu_topology::VENDOR_AMD,
        device_id: 0xffff,
        class_code: gpu_topology::PCI_CLASS_DISPLAY,
        subclass: 0,
        programming_interface: 0,
        revision: 0,
        header_type: 0,
    };
    if !is_amd_display(sample) || NativeGpuVendor::Amd as u8 != 1 {
        return Err("AMD native backend classifier self-test failed");
    }
    if !NativeDriverPhase::Quarantined.may_transition_to(NativeDriverPhase::Claimed)
        || NativeDriverPhase::Claimed.may_transition_to(NativeDriverPhase::AccelerationReady)
    {
        return Err("AMD backend lifecycle self-test failed");
    }
    Ok(())
}
