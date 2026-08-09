//! K14.C3 AMD bare-metal bring-up staging.
//!
//! C2 proved a persistent translated-domain lifetime with a surrogate requester.
//! C3 prepares the first physical Radeon path without weakening the safety model:
//! an actual AMD display function must already be owned by ForgeBus, its exact
//! requester must have a persistent hardware-translated IOMMU domain, and bus
//! mastering must remain off until that domain is live.  QEMU has no Radeon, so
//! the runtime qualification exercises the staging contract and requires the
//! native path to remain fenced while VirtIO-GPU/GOP stay available.

use crate::{
    k11_backends,
    native_gpu::{NativeGpuVendor, NativeIommuReadiness},
    native_gpu_binding,
    native_gpu_c2,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C3_ABI_VERSION: u32 = 1;
pub const AMD_IP_BLOCK_COUNT: u8 = 7;
pub const AMD_FIRMWARE_ROLE_COUNT: u8 = 6;
pub const AMD_MMIO_MINIMUM_WINDOW: u64 = 4096;
pub const AMD_DOORBELL_PAGE_BYTES: u64 = 4096;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmdIpBlock {
    Psp = 1,
    Smu = 2,
    Gmc = 3,
    InterruptHandler = 4,
    Sdma = 5,
    GraphicsCompute = 6,
    DisplayCore = 7,
}

pub const AMD_IP_BRINGUP_ORDER: [AmdIpBlock; AMD_IP_BLOCK_COUNT as usize] = [
    AmdIpBlock::Psp,
    AmdIpBlock::Smu,
    AmdIpBlock::Gmc,
    AmdIpBlock::InterruptHandler,
    AmdIpBlock::Sdma,
    AmdIpBlock::GraphicsCompute,
    AmdIpBlock::DisplayCore,
];

#[derive(Clone, Copy, Debug)]
pub struct C3State {
    pub amd_present: bool,
    pub selected_bus: u8,
    pub selected_device: u8,
    pub selected_function: u8,
    pub forge_claimed: bool,
    pub translation_engine_qualified: bool,
    pub amd_vi_live: bool,
    pub actual_gpu_domain_bound: bool,
    pub bar_inventory_ready: bool,
    pub mmio_mapping_authorized: bool,
    pub firmware_upload_authorized: bool,
    pub command_submission_authorized: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub bare_metal_required: bool,
}

impl C3State {
    pub const EMPTY: Self = Self {
        amd_present: false,
        selected_bus: 0,
        selected_device: 0,
        selected_function: 0,
        forge_claimed: false,
        translation_engine_qualified: false,
        amd_vi_live: false,
        actual_gpu_domain_bound: false,
        bar_inventory_ready: false,
        mmio_mapping_authorized: false,
        firmware_upload_authorized: false,
        command_submission_authorized: false,
        bus_master_enabled: false,
        fallback_armed: true,
        bare_metal_required: true,
    };
}

static STATE: SpinLock<C3State> = SpinLock::new(C3State::EMPTY);

fn self_test() -> Result<(), &'static str> {
    if AMD_IP_BRINGUP_ORDER.len() != AMD_IP_BLOCK_COUNT as usize
        || AMD_FIRMWARE_ROLE_COUNT != 6
        || AMD_MMIO_MINIMUM_WINDOW < 4096
        || AMD_DOORBELL_PAGE_BYTES != 4096
    {
        return Err("K14.C3 AMD bring-up constants are inconsistent");
    }
    if AMD_IP_BRINGUP_ORDER[0] != AmdIpBlock::Psp
        || AMD_IP_BRINGUP_ORDER[2] != AmdIpBlock::Gmc
        || AMD_IP_BRINGUP_ORDER[AMD_IP_BRINGUP_ORDER.len() - 1] != AmdIpBlock::DisplayCore
    {
        return Err("K14.C3 AMD IP bring-up ordering is invalid");
    }
    Ok(())
}

pub fn initialize() -> Result<C3State, &'static str> {
    self_test()?;
    let binding = native_gpu_binding::state();
    let c2 = native_gpu_c2::state();
    let amd_present = binding.selected_vendor == NativeGpuVendor::Amd as u8;
    let translation_engine_qualified = binding.translation_engine_qualified
        && crate::native_gpu::current_iommu_readiness() == NativeIommuReadiness::HardwareTranslated;
    let amd_vi_live = k11_backends::active_iommu() == k11_backends::ActiveIommu::AmdVi;

    serial::println(format_args!(
        "[C3IP] AMD IP bring-up graph: psp->smu->gmc->ih->sdma->gc->dcn blocks={} firmware_roles={} vendor_writes=false",
        AMD_IP_BLOCK_COUNT, AMD_FIRMWARE_ROLE_COUNT
    ));
    serial::println(format_args!(
        "[C3FW] Radeon firmware manifest contract: vbios=true psp=true smu=true gc=true sdma=true scheduler_mes=true upload=false signature_policy=required"
    ));
    serial::println(format_args!(
        "[C3MM] Radeon aperture policy: minimum_window={} doorbell_page={} map_after_domain=true bus_master_before_domain=false",
        AMD_MMIO_MINIMUM_WINDOW, AMD_DOORBELL_PAGE_BYTES
    ));

    let mut state = C3State {
        amd_present,
        selected_bus: binding.selected_bus,
        selected_device: binding.selected_device,
        selected_function: binding.selected_function,
        forge_claimed: binding.forge_claimed,
        translation_engine_qualified,
        amd_vi_live,
        actual_gpu_domain_bound: c2.actual_gpu_domain_bound,
        bar_inventory_ready: binding.bar_inventory_ready,
        mmio_mapping_authorized: false,
        firmware_upload_authorized: false,
        command_submission_authorized: false,
        bus_master_enabled: false,
        fallback_armed: true,
        bare_metal_required: true,
    };

    if !amd_present {
        serial::println(format_args!(
            "[C3HW] physical Radeon bring-up: present=false qemu_deferred=true actual_domain=false mmio=false firmware=false submit=false fallback=true"
        ));
    } else {
        let command = pci::read_u16(binding.selected_bus, binding.selected_device, binding.selected_function, 0x04);
        if command & (1 << 2) != 0 {
            return Err("physical Radeon entered K14.C3 with bus mastering already enabled");
        }
        if !binding.forge_claimed || !binding.bar_inventory_ready {
            return Err("physical Radeon lacks ForgeBus ownership or BAR inventory");
        }
        if !translation_engine_qualified {
            serial::println(format_args!(
                "[C3HW] physical Radeon bring-up: present=true at={:02x}:{:02x}.{} forge_claimed=true engine_qualified=false actual_domain=false bus_master=false fallback=true",
                binding.selected_bus, binding.selected_device, binding.selected_function
            ));
        } else if amd_vi_live && !c2.actual_gpu_domain_bound {
            // On an AMD host, K14.C3 must not confuse the K14.B Intel/QEMU proof
            // with an IVRS-backed translated domain for the actual Radeon RID.
            serial::println(format_args!(
                "[C3HW] physical Radeon bring-up: present=true at={:02x}:{:02x}.{} amd_vi=true actual_domain=false reason=amd_vi_hardware_domain_not_yet_bound bus_master=false fallback=true",
                binding.selected_bus, binding.selected_device, binding.selected_function
            ));
        } else {
            serial::println(format_args!(
                "[C3HW] physical Radeon bring-up: present=true at={:02x}:{:02x}.{} engine_qualified=true actual_domain={} mmio=false firmware=false submit=false bus_master=false fallback=true",
                binding.selected_bus, binding.selected_device, binding.selected_function, c2.actual_gpu_domain_bound
            ));
        }
    }

    // C3 is intentionally staging-only until the exact Radeon requester owns
    // a persistent translated domain.  These gates must all move together.
    if state.actual_gpu_domain_bound {
        state.mmio_mapping_authorized = true;
        // Firmware upload and command submission remain separate later gates.
    }
    state.bus_master_enabled = false;
    state.firmware_upload_authorized = false;
    state.command_submission_authorized = false;

    serial::println(format_args!(
        "[C3RD] K14.C3 Radeon bare-metal staging ready: amd_present={} forge_claimed={} engine_qualified={} amd_vi={} actual_domain={} mmio_authorized={} firmware_upload=false command_submit=false bus_master=false fallback=true",
        state.amd_present, state.forge_claimed, state.translation_engine_qualified,
        state.amd_vi_live, state.actual_gpu_domain_bound, state.mmio_mapping_authorized
    ));
    *STATE.lock() = state;
    Ok(state)
}

#[must_use]
pub fn state() -> C3State { *STATE.lock() }

/// Packed status for DISPLAYD.
/// bit0 AMD present, bit1 ForgeBus claimed, bit2 translation engine qualified,
/// bit3 AMD-Vi active, bit4 actual GPU domain, bit5 MMIO authorized,
/// bit6 firmware upload authorized, bit7 command submission authorized,
/// bit8 bus master enabled, bit9 fallback armed.
#[must_use]
pub fn packed_status() -> u64 {
    let s = state();
    let mut v = 0u64;
    if s.amd_present { v |= 1 << 0; }
    if s.forge_claimed { v |= 1 << 1; }
    if s.translation_engine_qualified { v |= 1 << 2; }
    if s.amd_vi_live { v |= 1 << 3; }
    if s.actual_gpu_domain_bound { v |= 1 << 4; }
    if s.mmio_mapping_authorized { v |= 1 << 5; }
    if s.firmware_upload_authorized { v |= 1 << 6; }
    if s.command_submission_authorized { v |= 1 << 7; }
    if s.bus_master_enabled { v |= 1 << 8; }
    if s.fallback_armed { v |= 1 << 9; }
    v
}
