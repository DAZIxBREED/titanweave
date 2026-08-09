//! K14.C4 AMD-Vi/Radeon exact-requester qualification gate.
//!
//! C4 is the first bare-metal gate that ties the selected Radeon BDF to the
//! AMD-Vi/IVRS path. It is deliberately fail-closed: QEMU has no Radeon and
//! therefore only exercises the contract. On bare metal, the exact requester
//! must be discovered, owned by ForgeBus, covered by AMD-Vi, and associated
//! with a persistent translated-domain plan before MMIO, firmware upload,
//! command submission, or PCI bus mastering may be promoted.

use crate::{
    k11_backends,
    native_gpu::{NativeGpuVendor, NativeIommuReadiness},
    native_gpu_binding,
    native_gpu_c3,
    pci,
    pci_address::{PciAddress, RequesterId},
    serial,
    sync::SpinLock,
};

pub const K14C4_ABI_VERSION: u32 = 1;
pub const AMD_VI_REQUIRED_FOR_AMD_HOST: bool = true;
pub const RADEON_DOMAIN_MIN_IOVA_PAGES: u32 = 16;
pub const RADEON_MMIO_STAGE_WRITES_ALLOWED: bool = false;
pub const RADEON_FIRMWARE_STAGE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_COMMAND_STAGE_SUBMIT_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct C4State {
    pub amd_present: bool,
    pub requester: RequesterId,
    pub forge_claimed: bool,
    pub amd_vi_active: bool,
    pub ivrs_unit_present: bool,
    pub requester_domain_planned: bool,
    pub persistent_domain_live: bool,
    pub mmio_read_mapping_allowed: bool,
    pub mmio_write_allowed: bool,
    pub firmware_upload_allowed: bool,
    pub command_submit_allowed: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
}

impl C4State {
    pub const EMPTY: Self = Self {
        amd_present: false,
        requester: RequesterId(0),
        forge_claimed: false,
        amd_vi_active: false,
        ivrs_unit_present: false,
        requester_domain_planned: false,
        persistent_domain_live: false,
        mmio_read_mapping_allowed: false,
        mmio_write_allowed: false,
        firmware_upload_allowed: false,
        command_submit_allowed: false,
        bus_master_enabled: false,
        fallback_armed: true,
    };
}

static STATE: SpinLock<C4State> = SpinLock::new(C4State::EMPTY);

fn self_test() -> Result<(), &'static str> {
    if K14C4_ABI_VERSION != 1
        || RADEON_DOMAIN_MIN_IOVA_PAGES < 16
        || RADEON_MMIO_STAGE_WRITES_ALLOWED
        || RADEON_FIRMWARE_STAGE_UPLOAD_ALLOWED
        || RADEON_COMMAND_STAGE_SUBMIT_ALLOWED
    {
        return Err("K14.C4 fail-closed constants are invalid");
    }
    Ok(())
}

pub fn initialize() -> Result<C4State, &'static str> {
    self_test()?;
    let binding = native_gpu_binding::state();
    let c3 = native_gpu_c3::state();
    let amd_present = binding.selected_vendor == NativeGpuVendor::Amd as u8;
    let amd_vi_active = k11_backends::active_iommu() == k11_backends::ActiveIommu::AmdVi;
    let ivrs_unit_present = k11_backends::amd_primary_register_base().is_some();
    let hardware_translation_seen = crate::native_gpu::current_iommu_readiness()
        == NativeIommuReadiness::HardwareTranslated;

    let requester = if amd_present {
        PciAddress::new(0, binding.selected_bus, binding.selected_device, binding.selected_function)?
            .requester_id()
    } else {
        RequesterId(0)
    };

    serial::println(format_args!(
        "[C4IV] AMD-Vi exact-requester gate: amd_present={} amd_vi_active={} ivrs_unit={} requester={:#06x} hardware_translation_seen={}",
        amd_present, amd_vi_active, ivrs_unit_present, requester.0, hardware_translation_seen
    ));
    serial::println(format_args!(
        "[C4DM] Radeon domain policy: exact_rid=true minimum_iova_pages={} persistent=true default_deny=true bus_master_after_domain=true",
        RADEON_DOMAIN_MIN_IOVA_PAGES
    ));
    serial::println(format_args!(
        "[C4AP] Radeon aperture promotion policy: read_map_after_domain=true mmio_write=false firmware_upload=false command_submit=false fallback=true"
    ));

    let mut state = C4State {
        amd_present,
        requester,
        forge_claimed: binding.forge_claimed,
        amd_vi_active,
        ivrs_unit_present,
        requester_domain_planned: false,
        persistent_domain_live: false,
        mmio_read_mapping_allowed: false,
        mmio_write_allowed: false,
        firmware_upload_allowed: false,
        command_submit_allowed: false,
        bus_master_enabled: false,
        fallback_armed: true,
    };

    if !amd_present {
        serial::println(format_args!(
            "[C4HW] physical Radeon domain bind: present=false qemu_deferred=true persistent_domain=false bus_master=false"
        ));
    } else {
        let command = pci::read_u16(
            binding.selected_bus,
            binding.selected_device,
            binding.selected_function,
            0x04,
        );
        if command & (1 << 2) != 0 {
            return Err("Radeon entered K14.C4 with bus mastering enabled");
        }
        if !binding.forge_claimed || !binding.bar_inventory_ready {
            return Err("Radeon lacks ForgeBus ownership or BAR inventory");
        }
        state.requester_domain_planned = true;

        if AMD_VI_REQUIRED_FOR_AMD_HOST && (!amd_vi_active || !ivrs_unit_present) {
            serial::println(format_args!(
                "[C4HW] physical Radeon domain bind: present=true rid={:#06x} planned=true amd_vi_ready=false persistent_domain=false bus_master=false fallback=true",
                requester.0
            ));
        } else {
            // C4 intentionally does not claim a live domain until a hardware AMD-Vi
            // page-table backend exists for this exact requester. The current
            // retained amd_vi module is the IVRS/default-deny software contract,
            // not yet a production hardware translation engine.
            serial::println(format_args!(
                "[C4HW] physical Radeon domain bind: present=true rid={:#06x} planned=true amd_vi_ready=true persistent_domain=false reason=hardware_amd_vi_page_tables_not_yet_live bus_master=false fallback=true",
                requester.0
            ));
        }
    }

    // Read-only aperture mapping is not promoted merely because C3 could stage
    // an aperture. C4 requires the exact requester domain to be live first.
    if state.persistent_domain_live && c3.mmio_mapping_authorized {
        state.mmio_read_mapping_allowed = true;
    }

    if state.bus_master_enabled || state.mmio_write_allowed || state.firmware_upload_allowed
        || state.command_submit_allowed
    {
        return Err("K14.C4 promoted a destructive Radeon capability too early");
    }

    serial::println(format_args!(
        "[C4RD] K14.C4 Radeon exact-domain gate ready: amd_present={} forge_claimed={} amd_vi={} ivrs={} domain_planned={} domain_live={} read_mmio={} write_mmio=false firmware=false submit=false bus_master=false fallback=true",
        state.amd_present, state.forge_claimed, state.amd_vi_active, state.ivrs_unit_present,
        state.requester_domain_planned, state.persistent_domain_live, state.mmio_read_mapping_allowed
    ));
    *STATE.lock() = state;
    Ok(state)
}

#[must_use]
pub fn state() -> C4State { *STATE.lock() }

/// Packed DISPLAYD status:
/// bit0 AMD present, bit1 ForgeBus claimed, bit2 AMD-Vi active, bit3 IVRS unit,
/// bit4 domain planned, bit5 persistent domain live, bit6 read-MMIO allowed,
/// bit7 MMIO writes, bit8 firmware upload, bit9 command submit,
/// bit10 bus master, bit11 fallback armed; requester RID in bits 16..31.
#[must_use]
pub fn packed_status() -> u64 {
    let s = state();
    let mut v = u64::from(s.requester.0) << 16;
    if s.amd_present { v |= 1 << 0; }
    if s.forge_claimed { v |= 1 << 1; }
    if s.amd_vi_active { v |= 1 << 2; }
    if s.ivrs_unit_present { v |= 1 << 3; }
    if s.requester_domain_planned { v |= 1 << 4; }
    if s.persistent_domain_live { v |= 1 << 5; }
    if s.mmio_read_mapping_allowed { v |= 1 << 6; }
    if s.mmio_write_allowed { v |= 1 << 7; }
    if s.firmware_upload_allowed { v |= 1 << 8; }
    if s.command_submit_allowed { v |= 1 << 9; }
    if s.bus_master_enabled { v |= 1 << 10; }
    if s.fallback_armed { v |= 1 << 11; }
    v
}
