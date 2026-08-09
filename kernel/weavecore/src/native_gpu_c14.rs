//! K14.C14 controlled Radeon write-promotion readiness gate.
//!
//! C13 established a durable physical read-proof contract. C14 still performs
//! no Radeon register write, firmware upload, command submission, or bus-master
//! enable. Instead it aggregates every prerequisite that must be true before a
//! later milestone is permitted to introduce the first carefully reviewed
//! write-side action.
//!
//! The gate is intentionally fail-closed. QEMU has no physical Radeon, so the
//! promotion result remains deferred there. On bare metal, eligibility requires
//! an exact verified profile, C13 physical proof, a live translated DMA domain,
//! trusted register aperture/base provenance, a rechecked-disabled PCI bus
//! master bit, and a nonzero immutable proof fingerprint.

use crate::{
    native_gpu_c6,
    native_gpu_c9,
    native_gpu_c12,
    native_gpu_c13,
    native_gpu_binding,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C14_ABI_VERSION: u32 = 1;
pub const RADEON_C14_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C14_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C14_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C14_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C14_WRITE_PROMOTION_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct C14State {
    pub amd_present: bool,
    pub profile_verified: bool,
    pub exact_domain_live: bool,
    pub trusted_base_source: bool,
    pub register_bar_ready: bool,
    pub physical_read_proof: bool,
    pub proof_fingerprint_present: bool,
    pub bus_master_rechecked_off: bool,
    pub write_prerequisites_complete: bool,
    pub write_promotion_enabled: bool,
    pub navi48_discovery_pending: bool,
    pub device_id: u16,
    pub revision: u8,
    pub fallback_armed: bool,
}

impl C14State {
    pub const EMPTY: Self = Self {
        amd_present: false,
        profile_verified: false,
        exact_domain_live: false,
        trusted_base_source: false,
        register_bar_ready: false,
        physical_read_proof: false,
        proof_fingerprint_present: false,
        bus_master_rechecked_off: false,
        write_prerequisites_complete: false,
        write_promotion_enabled: false,
        navi48_discovery_pending: false,
        device_id: 0,
        revision: 0,
        fallback_armed: true,
    };
}

static STATE: SpinLock<C14State> = SpinLock::new(C14State::EMPTY);

fn self_test() -> Result<(), &'static str> {
    if K14C14_ABI_VERSION != 1
        || RADEON_C14_MMIO_WRITES_ALLOWED
        || RADEON_C14_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C14_COMMAND_SUBMIT_ALLOWED
        || RADEON_C14_BUS_MASTER_ALLOWED
        || RADEON_C14_WRITE_PROMOTION_ALLOWED
    {
        return Err("K14.C14 fail-closed promotion constants invalid");
    }
    Ok(())
}

pub fn initialize() -> Result<C14State, &'static str> {
    self_test()?;

    let c6 = native_gpu_c6::state();
    let c9 = native_gpu_c9::state();
    let c12 = native_gpu_c12::state();
    let c13 = native_gpu_c13::state();

    let mut s = C14State {
        amd_present: c9.amd_present,
        profile_verified: c9.profile_verified && c9.pci_identity_consistent,
        exact_domain_live: c6.persistent_domain_live,
        trusted_base_source: c12.gc_base_ready && c12.sdma_base_ready,
        register_bar_ready: c12.register_mmio_bar_ready,
        physical_read_proof: c13.physical_proof_complete,
        proof_fingerprint_present: c13.proof_fingerprint != 0,
        navi48_discovery_pending: c13.navi48_discovery_pending,
        device_id: c9.device_id,
        revision: c9.revision,
        ..C14State::EMPTY
    };

    serial::println(format_args!(
        "[C14PG] write-promotion policy: exact_profile=true translated_domain=true trusted_ip_bases=true BAR5=true C13_physical_proof=true fingerprint=true bus_master_off=true actual_writes=false firmware_upload=false submit=false"
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C14HW] Radeon write-promotion readiness: present=false qemu_deferred=true eligible=false promotion=false fallback=true"
        ));
    } else {
        let b = native_gpu_binding::state();
        let command = pci::read_u16(
            b.selected_bus,
            b.selected_device,
            b.selected_function,
            0x04,
        );
        s.bus_master_rechecked_off = command & (1 << 2) == 0;

        if !s.bus_master_rechecked_off {
            return Err("K14.C14 Radeon bus mastering unexpectedly enabled");
        }

        s.write_prerequisites_complete =
            s.profile_verified
            && s.exact_domain_live
            && s.trusted_base_source
            && s.register_bar_ready
            && s.physical_read_proof
            && s.proof_fingerprint_present
            && s.bus_master_rechecked_off
            && !s.navi48_discovery_pending;

        serial::println(format_args!(
            "[C14CK] promotion prerequisite check: profile={} domain={} trusted_bases={} bar5={} physical_proof={} fingerprint={} bus_master_off={} navi48_pending={} eligible={}",
            s.profile_verified,
            s.exact_domain_live,
            s.trusted_base_source,
            s.register_bar_ready,
            s.physical_read_proof,
            s.proof_fingerprint_present,
            s.bus_master_rechecked_off,
            s.navi48_discovery_pending,
            s.write_prerequisites_complete
        ));

        serial::println(format_args!(
            "[C14HW] Radeon write-promotion readiness: present=true devid={:#06x} revision={:#04x} eligible={} promotion=false writes=false firmware_upload=false submit=false bus_master=false fallback=true",
            s.device_id,
            s.revision,
            s.write_prerequisites_complete
        ));
    }

    // C14 may prove eligibility, but it must never promote the write path.
    s.write_promotion_enabled = false;

    if s.write_promotion_enabled
        || RADEON_C14_WRITE_PROMOTION_ALLOWED
        || RADEON_C14_MMIO_WRITES_ALLOWED
        || RADEON_C14_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C14_COMMAND_SUBMIT_ALLOWED
        || RADEON_C14_BUS_MASTER_ALLOWED
    {
        return Err("K14.C14 destructive Radeon capability promoted early");
    }

    if s.write_prerequisites_complete && !s.physical_read_proof {
        return Err("K14.C14 write eligibility without C13 physical proof");
    }

    serial::println(format_args!(
        "[C14RD] K14.C14 write-promotion readiness ready: amd_present={} profile={} domain={} trusted_bases={} bar5={} physical_proof={} fingerprint={} bus_master_off={} prerequisites={} promotion=false writes=false upload=false submit=false fallback=true",
        s.amd_present,
        s.profile_verified,
        s.exact_domain_live,
        s.trusted_base_source,
        s.register_bar_ready,
        s.physical_read_proof,
        s.proof_fingerprint_present,
        s.bus_master_rechecked_off,
        s.write_prerequisites_complete
    ));

    *STATE.lock() = s;
    Ok(s)
}

pub fn state() -> C14State {
    *STATE.lock()
}

pub fn packed_status() -> u64 {
    let s = state();
    let mut v =
        (u64::from(s.device_id) << 32)
        | (u64::from(s.revision) << 24);

    for (bit, on) in [
        s.amd_present,
        s.profile_verified,
        s.exact_domain_live,
        s.trusted_base_source,
        s.register_bar_ready,
        s.physical_read_proof,
        s.proof_fingerprint_present,
        s.bus_master_rechecked_off,
        s.write_prerequisites_complete,
        s.write_promotion_enabled,
        s.navi48_discovery_pending,
        s.fallback_armed,
    ]
    .into_iter()
    .enumerate()
    {
        if on {
            v |= 1u64 << bit;
        }
    }
    v
}
