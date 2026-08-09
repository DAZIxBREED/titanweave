//! K14.C7 controlled Radeon MMIO / firmware-discovery staging.
//!
//! C7 is the first Radeon-side slice after the AMD-Vi protection boundary.
//! It may promote a *read-only* supervisor MMIO mapping only when C6 proves a
//! persistent translated domain for the exact Radeon requester.  It does not
//! perform vendor register writes, enable bus mastering, upload firmware, or
//! submit commands.  QEMU has no physical Radeon, so qualification requires
//! the entire native path to remain fenced while the contract is exercised.

use crate::{
    amd_gpu,
    memory::FrameAllocator,
    native_gpu::{NativeGpuVendor},
    native_gpu_binding,
    native_gpu_c6,
    paging,
    pci::{self, PciFunction},
    serial,
    sync::SpinLock,
};

pub const K14C7_ABI_VERSION: u32 = 1;
pub const RADEON_C7_PROBE_BYTES: u64 = 4096;
pub const RADEON_C7_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C7_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C7_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C7_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C7_REGISTER_READS_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct C7State {
    pub amd_present: bool,
    pub exact_domain_live: bool,
    pub bar_inventory_ready: bool,
    pub probe_bar_index: u8,
    pub probe_bar_phys: u64,
    pub read_only_mmio_mapped: bool,
    pub read_only_mmio_virt: u64,
    pub pci_identity_ready: bool,
    pub vbios_discovery_planned: bool,
    pub firmware_manifest_ready: bool,
    pub gmc_gtt_readiness_planned: bool,
    pub register_reads_enabled: bool,
    pub register_writes_enabled: bool,
    pub firmware_upload_enabled: bool,
    pub command_submit_enabled: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub vendor_id: u16,
    pub device_id: u16,
    pub revision: u8,
}

impl C7State {
    pub const EMPTY: Self = Self {
        amd_present: false,
        exact_domain_live: false,
        bar_inventory_ready: false,
        probe_bar_index: 0xff,
        probe_bar_phys: 0,
        read_only_mmio_mapped: false,
        read_only_mmio_virt: 0,
        pci_identity_ready: false,
        vbios_discovery_planned: false,
        firmware_manifest_ready: false,
        gmc_gtt_readiness_planned: false,
        register_reads_enabled: false,
        register_writes_enabled: false,
        firmware_upload_enabled: false,
        command_submit_enabled: false,
        bus_master_enabled: false,
        fallback_armed: true,
        vendor_id: 0,
        device_id: 0,
        revision: 0,
    };
}

static STATE: SpinLock<C7State> = SpinLock::new(C7State::EMPTY);

fn selected_function() -> Option<PciFunction> {
    let b = native_gpu_binding::state();
    let mut found = None;
    pci::enumerate(|f| {
        if f.bus == b.selected_bus && f.device == b.selected_device && f.function == b.selected_function {
            found = Some(f);
        }
    });
    found
}

fn first_memory_bar(function: PciFunction) -> Option<(u8, u64)> {
    let bars = amd_gpu::inventory_bars(function);
    let mut index = 0u8;
    while index < 6 {
        let base = bars.bars[index as usize];
        if base != 0 { return Some((index, base)); }
        index = index.saturating_add(1);
    }
    None
}

fn self_test() -> Result<(), &'static str> {
    if K14C7_ABI_VERSION != 1 || RADEON_C7_PROBE_BYTES != 4096
        || RADEON_C7_MMIO_WRITES_ALLOWED || RADEON_C7_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C7_COMMAND_SUBMIT_ALLOWED || RADEON_C7_BUS_MASTER_ALLOWED
        || RADEON_C7_REGISTER_READS_ALLOWED
    {
        return Err("K14.C7 fail-closed constants invalid");
    }
    Ok(())
}

pub fn initialize(allocator: &mut FrameAllocator<'_>, kernel_cr3: u64) -> Result<C7State, &'static str> {
    self_test()?;
    let binding = native_gpu_binding::state();
    let c6 = native_gpu_c6::state();
    let amd_present = binding.selected_vendor == NativeGpuVendor::Amd as u8;
    let mut state = C7State { amd_present, exact_domain_live: c6.persistent_domain_live,
        bar_inventory_ready: binding.bar_inventory_ready, ..C7State::EMPTY };

    serial::println(format_args!(
        "[C7AP] Radeon read-only aperture policy: probe_bytes={} require_exact_domain=true supervisor_only=true uncached=true executable=false register_reads=false writes=false",
        RADEON_C7_PROBE_BYTES
    ));
    serial::println(format_args!(
        "[C7FW] Radeon discovery sequence: pci_identity -> read_only_aperture -> vbios_locator -> asic_ip_manifest -> firmware_manifest -> gmc_gtt_readiness; upload=false"
    ));
    serial::println(format_args!(
        "[C7GT] GMC/GTT promotion policy: translated_domain -> identity -> firmware_requirements -> memory_topology -> later_writable_register_gate; bus_master=false submit=false"
    ));

    if !amd_present {
        serial::println(format_args!(
            "[C7HW] physical Radeon discovery: present=false qemu_deferred=true ro_mmio=false register_reads=false firmware=false gmc_gtt=false writes=false submit=false bus_master=false fallback=true"
        ));
    } else {
        let function = selected_function().ok_or("K14.C7 selected Radeon disappeared from PCI inventory")?;
        if function.vendor_id != crate::gpu_topology::VENDOR_AMD {
            return Err("K14.C7 selected function is not AMD");
        }
        let command = pci::read_u16(function.bus, function.device, function.function, 0x04);
        if command & (1 << 2) != 0 { return Err("K14.C7 Radeon bus mastering unexpectedly enabled"); }
        state.vendor_id = function.vendor_id;
        state.device_id = function.device_id;
        state.revision = function.revision;
        state.pci_identity_ready = true;
        state.vbios_discovery_planned = true;
        state.firmware_manifest_ready = true;
        state.gmc_gtt_readiness_planned = true;

        if !c6.persistent_domain_live || !c6.read_only_radeon_mmio_promoted {
            serial::println(format_args!(
                "[C7HW] physical Radeon discovery: present=true at={:02x}:{:02x}.{} devid={:#06x} domain_live=false ro_mmio=false reason=c6_exact_domain_not_live register_reads=false writes=false firmware=false submit=false bus_master=false fallback=true",
                function.bus, function.device, function.function, function.device_id
            ));
        } else if let Some((index, base)) = first_memory_bar(function) {
            state.probe_bar_index = index;
            state.probe_bar_phys = base;
            state.read_only_mmio_virt = paging::map_kernel_mmio_readonly(allocator, kernel_cr3, base, RADEON_C7_PROBE_BYTES)?;
            state.read_only_mmio_mapped = true;
            serial::println(format_args!(
                "[C7HW] physical Radeon discovery: present=true at={:02x}:{:02x}.{} devid={:#06x} domain_live=true bar={} bar_phys={:#x} ro_mmio=true register_reads=false writes=false firmware=false submit=false bus_master=false fallback=true",
                function.bus, function.device, function.function, function.device_id, index, base
            ));
        } else {
            return Err("K14.C7 Radeon has no usable memory BAR");
        }
    }

    if state.read_only_mmio_mapped && !state.exact_domain_live {
        return Err("K14.C7 mapped Radeon MMIO before exact translated domain");
    }
    if state.register_reads_enabled || state.register_writes_enabled || state.firmware_upload_enabled
        || state.command_submit_enabled || state.bus_master_enabled || RADEON_C7_MMIO_WRITES_ALLOWED
        || RADEON_C7_FIRMWARE_UPLOAD_ALLOWED || RADEON_C7_COMMAND_SUBMIT_ALLOWED || RADEON_C7_BUS_MASTER_ALLOWED
    {
        return Err("K14.C7 destructive Radeon capability promoted early");
    }

    serial::println(format_args!(
        "[C7RD] K14.C7 Radeon discovery ready: amd_present={} domain_live={} bars={} pci_identity={} ro_mmio={} vbios_plan={} firmware_manifest={} gmc_gtt_plan={} register_reads=false writes=false upload=false submit=false bus_master=false fallback=true",
        state.amd_present, state.exact_domain_live, state.bar_inventory_ready, state.pci_identity_ready,
        state.read_only_mmio_mapped, state.vbios_discovery_planned, state.firmware_manifest_ready,
        state.gmc_gtt_readiness_planned
    ));
    *STATE.lock() = state;
    Ok(state)
}

pub fn state() -> C7State { *STATE.lock() }

pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.device_id) << 32) | (u64::from(s.probe_bar_index) << 24);
    for (bit, on) in [s.amd_present, s.exact_domain_live, s.bar_inventory_ready, s.pci_identity_ready,
        s.read_only_mmio_mapped, s.vbios_discovery_planned, s.firmware_manifest_ready,
        s.gmc_gtt_readiness_planned, s.register_reads_enabled, s.register_writes_enabled,
        s.firmware_upload_enabled, s.command_submit_enabled, s.bus_master_enabled, s.fallback_armed]
        .into_iter().enumerate() { if on { v |= 1u64 << bit; } }
    v
}
