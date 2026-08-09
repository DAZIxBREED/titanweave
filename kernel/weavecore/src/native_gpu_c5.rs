//! K14.C5 AMD-Vi hardware page-table engine foundation for a physical Radeon.
//!
//! C5 converts the C4 exact-requester policy into concrete, pinned AMD-Vi
//! translation structures: a device table, second-level page-table root,
//! command buffer and event log.  The structures are only allocated for a
//! physical AMD Radeon behind AMD-Vi.  Hardware programming remains fail-
//! closed until the exact platform's IVRS unit is qualified; QEMU therefore
//! exercises the contract without claiming a Radeon domain.

use crate::{
    amd_vi::{self, AmdViDomainImage},
    k11_backends,
    memory::FrameAllocator,
    native_gpu::NativeGpuVendor,
    native_gpu_binding,
    native_gpu_c4,
    serial,
    sync::SpinLock,
};

pub const K14C5_ABI_VERSION: u32 = 1;
pub const RADEON_C5_DOMAIN_ID: u16 = 0x14c5;
pub const RADEON_C5_IOVA_BITS: u8 = 48;
pub const RADEON_C5_HW_PROGRAMMING_DEFAULT: bool = false;
pub const RADEON_C5_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C5_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C5_COMMAND_SUBMIT_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct C5State {
    pub amd_present: bool,
    pub requester: u16,
    pub amd_vi_active: bool,
    pub page_tables_ready: bool,
    pub device_table_ready: bool,
    pub command_buffer_ready: bool,
    pub event_log_ready: bool,
    pub fault_path_ready: bool,
    pub exact_requester_bound: bool,
    pub persistent_domain_live: bool,
    pub mmio_read_mapping_allowed: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub domain_id: u16,
    pub device_table_phys: u64,
    pub page_table_root_phys: u64,
    pub command_buffer_phys: u64,
    pub event_log_phys: u64,
}

impl C5State {
    pub const EMPTY: Self = Self {
        amd_present: false, requester: 0, amd_vi_active: false,
        page_tables_ready: false, device_table_ready: false,
        command_buffer_ready: false, event_log_ready: false, fault_path_ready: false,
        exact_requester_bound: false, persistent_domain_live: false,
        mmio_read_mapping_allowed: false, bus_master_enabled: false,
        fallback_armed: true, domain_id: RADEON_C5_DOMAIN_ID,
        device_table_phys: 0, page_table_root_phys: 0, command_buffer_phys: 0, event_log_phys: 0,
    };
}

static STATE: SpinLock<C5State> = SpinLock::new(C5State::EMPTY);

fn self_test() -> Result<(), &'static str> {
    if K14C5_ABI_VERSION != 1 || RADEON_C5_IOVA_BITS != 48
        || RADEON_C5_HW_PROGRAMMING_DEFAULT || RADEON_C5_MMIO_WRITES_ALLOWED
        || RADEON_C5_FIRMWARE_UPLOAD_ALLOWED || RADEON_C5_COMMAND_SUBMIT_ALLOWED {
        return Err("K14.C5 fail-closed constants are invalid");
    }
    amd_vi::c5_layout_self_test()?;
    Ok(())
}

pub fn initialize(allocator: &mut FrameAllocator<'_>) -> Result<C5State, &'static str> {
    self_test()?;
    let binding = native_gpu_binding::state();
    let c4 = native_gpu_c4::state();
    let amd_present = binding.selected_vendor == NativeGpuVendor::Amd as u8;
    let amd_vi_active = k11_backends::active_iommu() == k11_backends::ActiveIommu::AmdVi;

    serial::println(format_args!(
        "[C5PT] AMD-Vi translation layout: dte_bytes={} command_entry_bytes={} event_entry_bytes={} iova_bits={} domain={:#06x}",
        amd_vi::AMDVI_DTE_BYTES, amd_vi::AMDVI_COMMAND_BYTES, amd_vi::AMDVI_EVENT_BYTES,
        RADEON_C5_IOVA_BITS, RADEON_C5_DOMAIN_ID
    ));
    serial::println(format_args!(
        "[C5CB] AMD-Vi command/event policy: pinned=true zeroed=true bounded=true completion_required=true event_faults_default_deny=true"
    ));

    let mut state = C5State {
        amd_present, requester: c4.requester.0, amd_vi_active, ..C5State::EMPTY
    };

    if !amd_present {
        serial::println(format_args!(
            "[C5HW] physical AMD-Vi domain image: present=false qemu_deferred=true tables=false exact_bound=false domain_live=false bus_master=false fallback=true"
        ));
    } else {
        if !c4.forge_claimed || !c4.requester_domain_planned {
            return Err("K14.C5 Radeon did not satisfy frozen C4 ownership/domain-plan gate");
        }
        if !amd_vi_active || !c4.ivrs_unit_present {
            serial::println(format_args!(
                "[C5HW] physical AMD-Vi domain image: present=true rid={:#06x} amd_vi=false tables=false exact_bound=false domain_live=false bus_master=false fallback=true",
                c4.requester.0
            ));
        } else {
            let image = AmdViDomainImage::allocate(allocator, c4.requester, RADEON_C5_DOMAIN_ID)?;
            state.page_tables_ready = image.page_table_root != 0;
            state.device_table_ready = image.device_table != 0;
            state.command_buffer_ready = image.command_buffer != 0;
            state.event_log_ready = image.event_log != 0;
            state.fault_path_ready = state.event_log_ready;
            state.device_table_phys = image.device_table;
            state.page_table_root_phys = image.page_table_root;
            state.command_buffer_phys = image.command_buffer;
            state.event_log_phys = image.event_log;

            // A concrete DTE is built for the exact requester, but we deliberately
            // do not program the physical AMD-Vi MMIO registers in the same step.
            // C5 bare-metal qualification must first confirm the discovered unit's
            // feature/version contract and command/event completion behavior.
            image.install_exact_requester_dte(c4.requester, RADEON_C5_DOMAIN_ID)?;
            state.exact_requester_bound = true;

            serial::println(format_args!(
                "[C5HW] physical AMD-Vi domain image: present=true rid={:#06x} amd_vi=true tables=true dte=true command=true event=true exact_bound=true domain_live=false reason=hardware_register_programming_requires_bare_metal_qualification bus_master=false fallback=true",
                c4.requester.0
            ));
        }
    }

    if state.persistent_domain_live {
        if !(state.page_tables_ready && state.device_table_ready && state.command_buffer_ready
            && state.event_log_ready && state.fault_path_ready && state.exact_requester_bound) {
            return Err("K14.C5 domain promoted without complete AMD-Vi structures");
        }
        state.mmio_read_mapping_allowed = true;
    }
    if state.bus_master_enabled || RADEON_C5_MMIO_WRITES_ALLOWED
        || RADEON_C5_FIRMWARE_UPLOAD_ALLOWED || RADEON_C5_COMMAND_SUBMIT_ALLOWED {
        return Err("K14.C5 promoted destructive Radeon capability too early");
    }

    serial::println(format_args!(
        "[C5RD] K14.C5 AMD-Vi page-table engine ready: amd_present={} amd_vi={} tables={} dte={} cmd={} event={} fault={} exact_bound={} domain_live={} read_mmio={} write_mmio=false firmware=false submit=false bus_master=false fallback=true",
        state.amd_present, state.amd_vi_active, state.page_tables_ready, state.device_table_ready,
        state.command_buffer_ready, state.event_log_ready, state.fault_path_ready,
        state.exact_requester_bound, state.persistent_domain_live, state.mmio_read_mapping_allowed
    ));
    *STATE.lock() = state;
    Ok(state)
}

#[must_use]
pub fn state() -> C5State { *STATE.lock() }

/// Packed DISPLAYD status: bits 0..10 are booleans, RID bits 16..31, domain 32..47.
#[must_use]
pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.requester) << 16) | (u64::from(s.domain_id) << 32);
    for (bit, on) in [
        s.amd_present, s.amd_vi_active, s.page_tables_ready, s.device_table_ready,
        s.command_buffer_ready, s.event_log_ready, s.fault_path_ready,
        s.exact_requester_bound, s.persistent_domain_live, s.mmio_read_mapping_allowed,
        s.fallback_armed,
    ].into_iter().enumerate() { if on { v |= 1u64 << bit; } }
    v
}
