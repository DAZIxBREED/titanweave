//! K14.C8 Radeon ASIC/IP identification and safe-register-read gate.
//!
//! C8 sits on top of C7's supervisor-only read-only MMIO aperture.  It does
//! not assume that a PCI vendor/device ID is enough to make arbitrary MMIO
//! reads safe.  Register reads are enabled only after the exact device matches
//! a verified ASIC profile whose offsets and access semantics have been
//! reviewed for that IP revision. Unknown physical ASICs remain fail-closed.

use crate::{native_gpu_c7, serial, sync::SpinLock};

pub const K14C8_ABI_VERSION: u32 = 1;
pub const RADEON_C8_REGISTER_WRITES_ALLOWED: bool = false;
pub const RADEON_C8_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C8_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C8_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C8_MAX_SAFE_READS: u8 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AsicProfileKind { Unknown = 0, Verified = 1 }

#[derive(Clone, Copy, Debug)]
pub struct SafeRegisterDescriptor {
    pub offset: u32,
    pub width: u8,
    pub read_has_side_effects: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct VerifiedAsicProfile {
    pub vendor_id: u16,
    pub device_id: u16,
    pub revision_min: u8,
    pub revision_max: u8,
    pub gc_ip_major: u8,
    pub gmc_ip_major: u8,
    pub sdma_ip_major: u8,
    pub dcn_ip_major: u8,
    pub safe_registers: &'static [SafeRegisterDescriptor],
}

// Intentionally empty until a profile is grounded in verified AMD/public
// hardware documentation for the exact ASIC/IP revision. C8 must never infer
// MMIO safety from marketing family names or a guessed PCI ID.
static VERIFIED_ASIC_PROFILES: &[VerifiedAsicProfile] = &[];

#[derive(Clone, Copy, Debug)]
pub struct C8State {
    pub amd_present: bool,
    pub c7_ro_mmio_ready: bool,
    pub pci_identity_ready: bool,
    pub asic_profile: AsicProfileKind,
    pub asic_profile_verified: bool,
    pub ip_manifest_ready: bool,
    pub safe_read_whitelist_ready: bool,
    pub safe_register_reads_enabled: bool,
    pub safe_reads_performed: u8,
    pub firmware_requirements_resolved: bool,
    pub gmc_gtt_init_ready: bool,
    pub register_writes_enabled: bool,
    pub firmware_upload_enabled: bool,
    pub command_submit_enabled: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub vendor_id: u16,
    pub device_id: u16,
    pub revision: u8,
    pub gc_ip_major: u8,
    pub gmc_ip_major: u8,
    pub sdma_ip_major: u8,
    pub dcn_ip_major: u8,
}

impl C8State {
    pub const EMPTY: Self = Self {
        amd_present:false, c7_ro_mmio_ready:false, pci_identity_ready:false,
        asic_profile:AsicProfileKind::Unknown, asic_profile_verified:false,
        ip_manifest_ready:false, safe_read_whitelist_ready:false,
        safe_register_reads_enabled:false, safe_reads_performed:0,
        firmware_requirements_resolved:false, gmc_gtt_init_ready:false,
        register_writes_enabled:false, firmware_upload_enabled:false,
        command_submit_enabled:false, bus_master_enabled:false,
        fallback_armed:true, vendor_id:0, device_id:0, revision:0,
        gc_ip_major:0, gmc_ip_major:0, sdma_ip_major:0, dcn_ip_major:0,
    };
}

static STATE: SpinLock<C8State> = SpinLock::new(C8State::EMPTY);

fn profile_for(vendor:u16, device:u16, revision:u8) -> Option<&'static VerifiedAsicProfile> {
    VERIFIED_ASIC_PROFILES.iter().find(|p| p.vendor_id==vendor && p.device_id==device
        && revision>=p.revision_min && revision<=p.revision_max
        && p.safe_registers.len() <= RADEON_C8_MAX_SAFE_READS as usize
        && p.safe_registers.iter().all(|r| !r.read_has_side_effects && matches!(r.width, 1|2|4|8)))
}

fn self_test() -> Result<(), &'static str> {
    if K14C8_ABI_VERSION != 1 || RADEON_C8_MAX_SAFE_READS == 0
        || RADEON_C8_REGISTER_WRITES_ALLOWED || RADEON_C8_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C8_COMMAND_SUBMIT_ALLOWED || RADEON_C8_BUS_MASTER_ALLOWED {
        return Err("K14.C8 fail-closed constants invalid");
    }
    Ok(())
}

pub fn initialize() -> Result<C8State, &'static str> {
    self_test()?;
    let c7 = native_gpu_c7::state();
    let mut s = C8State { amd_present:c7.amd_present, c7_ro_mmio_ready:c7.read_only_mmio_mapped,
        pci_identity_ready:c7.pci_identity_ready, vendor_id:c7.vendor_id, device_id:c7.device_id,
        revision:c7.revision, ..C8State::EMPTY };

    serial::println(format_args!(
        "[C8ID] Radeon ASIC/IP identity gate: require_c7_ro_mmio=true require_verified_profile=true guessed_ids=false unknown_asic=fenced"
    ));
    serial::println(format_args!(
        "[C8RR] safe register-read policy: whitelist_only=true side_effect_reads=false max_reads={} writes=false upload=false submit=false bus_master=false",
        RADEON_C8_MAX_SAFE_READS
    ));
    serial::println(format_args!(
        "[C8IP] IP manifest policy: verified_profile -> GC/GMC/SDMA/DCN revisions -> firmware requirements -> GMC/GTT readiness"
    ));

    if !c7.amd_present {
        serial::println(format_args!(
            "[C8HW] physical Radeon identification: present=false qemu_deferred=true profile=unknown safe_reads=false firmware_resolved=false gmc_gtt=false fallback=true"
        ));
    } else if !c7.read_only_mmio_mapped || !c7.pci_identity_ready {
        serial::println(format_args!(
            "[C8HW] physical Radeon identification: present=true devid={:#06x} rev={:#04x} c7_ro_mmio=false profile=unknown safe_reads=false reason=c7_not_promoted fallback=true",
            c7.device_id, c7.revision
        ));
    } else if let Some(p) = profile_for(c7.vendor_id, c7.device_id, c7.revision) {
        s.asic_profile = AsicProfileKind::Verified;
        s.asic_profile_verified = true;
        s.ip_manifest_ready = true;
        s.safe_read_whitelist_ready = true;
        s.gc_ip_major=p.gc_ip_major; s.gmc_ip_major=p.gmc_ip_major;
        s.sdma_ip_major=p.sdma_ip_major; s.dcn_ip_major=p.dcn_ip_major;
        // C8 foundation proves the policy and whitelist. Actual volatile reads
        // are promoted in the bare-metal follow-up after profile review.
        s.safe_register_reads_enabled = false;
        s.firmware_requirements_resolved = true;
        s.gmc_gtt_init_ready = true;
        serial::println(format_args!(
            "[C8HW] physical Radeon identification: present=true devid={:#06x} rev={:#04x} profile=verified gc={} gmc={} sdma={} dcn={} whitelist={} safe_reads=false activation=deferred fallback=true",
            s.device_id,s.revision,s.gc_ip_major,s.gmc_ip_major,s.sdma_ip_major,s.dcn_ip_major,p.safe_registers.len()
        ));
    } else {
        serial::println(format_args!(
            "[C8HW] physical Radeon identification: present=true devid={:#06x} rev={:#04x} profile=unknown safe_reads=false firmware_resolved=false gmc_gtt=false reason=no_verified_asic_profile fallback=true",
            s.device_id, s.revision
        ));
    }

    if s.safe_register_reads_enabled && (!s.c7_ro_mmio_ready || !s.asic_profile_verified || !s.safe_read_whitelist_ready) {
        return Err("K14.C8 enabled register reads without verified profile/read-only aperture");
    }
    if s.register_writes_enabled || s.firmware_upload_enabled || s.command_submit_enabled || s.bus_master_enabled {
        return Err("K14.C8 destructive Radeon capability promoted early");
    }

    serial::println(format_args!(
        "[C8RD] K14.C8 Radeon ASIC/IP identification ready: amd_present={} ro_mmio={} profile_verified={} ip_manifest={} whitelist={} safe_reads={} firmware_resolved={} gmc_gtt={} writes=false upload=false submit=false bus_master=false fallback=true",
        s.amd_present,s.c7_ro_mmio_ready,s.asic_profile_verified,s.ip_manifest_ready,
        s.safe_read_whitelist_ready,s.safe_register_reads_enabled,s.firmware_requirements_resolved,s.gmc_gtt_init_ready
    ));
    *STATE.lock()=s;
    Ok(s)
}

pub fn state() -> C8State { *STATE.lock() }
pub fn packed_status() -> u64 {
    let s=state();
    let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24);
    for (bit,on) in [s.amd_present,s.c7_ro_mmio_ready,s.pci_identity_ready,s.asic_profile_verified,
        s.ip_manifest_ready,s.safe_read_whitelist_ready,s.safe_register_reads_enabled,
        s.firmware_requirements_resolved,s.gmc_gtt_init_ready,s.register_writes_enabled,
        s.firmware_upload_enabled,s.command_submit_enabled,s.bus_master_enabled,s.fallback_armed]
        .into_iter().enumerate(){if on{v|=1u64<<bit;}}
    v
}
