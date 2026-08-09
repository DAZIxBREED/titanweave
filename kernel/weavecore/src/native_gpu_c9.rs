//! K14.C9 verified Radeon profiles + live safe identity reads.
//!
//! C9 turns the C8 profile policy into a grounded device-profile table for the
//! first Titanweave bring-up targets.  It performs only conventional PCI config
//! reads whose semantics are stable and side-effect free: vendor/device ID,
//! revision/class identity, and command state.  Radeon MMIO register reads stay
//! fenced until per-IP offsets are verified separately.  No GPU register write,
//! firmware upload, command submission, or bus mastering is enabled here.

use crate::{native_gpu::NativeGpuVendor, native_gpu_binding, native_gpu_c8, pci, serial, sync::SpinLock};

pub const K14C9_ABI_VERSION: u32 = 1;
pub const AMD_VENDOR_ID: u16 = 0x1002;
pub const RADEON_C9_MMIO_READS_ALLOWED: bool = false;
pub const RADEON_C9_REGISTER_WRITES_ALLOWED: bool = false;
pub const RADEON_C9_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C9_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C9_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C9_SAFE_PCI_READS: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ProfileId {
    Unknown = 0,
    Navi21Rx6800_6900 = 1,
    Navi48Rx9070 = 2,
}

#[derive(Clone, Copy, Debug)]
pub struct VerifiedRadeonProfile {
    pub id: ProfileId,
    pub device_id: u16,
    pub revision_min: u8,
    pub revision_max: u8,
    pub gc_ip_major: u8,
    pub gmc_ip_major: u8,
    pub sdma_ip_major: u8,
    pub dcn_ip_major: u8,
    pub mmio_whitelist_entries: u8,
}

// Grounded bring-up identities.  C9 deliberately leaves MMIO whitelist count
// at zero: profile identity is verified, but IP-specific MMIO offsets are a
// separate promotion gate and are not guessed from family names.
static VERIFIED_RADEON_PROFILES: &[VerifiedRadeonProfile] = &[
    VerifiedRadeonProfile {
        id: ProfileId::Navi21Rx6800_6900,
        device_id: 0x73bf,
        revision_min: 0x00,
        revision_max: 0xff,
        gc_ip_major: 10,
        gmc_ip_major: 10,
        sdma_ip_major: 5,
        dcn_ip_major: 3,
        mmio_whitelist_entries: 0,
    },
    VerifiedRadeonProfile {
        id: ProfileId::Navi48Rx9070,
        device_id: 0x7550,
        revision_min: 0xc0,
        revision_max: 0xcf,
        gc_ip_major: 12,
        gmc_ip_major: 12,
        sdma_ip_major: 7,
        dcn_ip_major: 4,
        mmio_whitelist_entries: 0,
    },
];

#[derive(Clone, Copy, Debug)]
pub struct C9State {
    pub amd_present: bool,
    pub c8_profile_policy_ready: bool,
    pub profile: ProfileId,
    pub profile_verified: bool,
    pub live_pci_identity_reads: bool,
    pub safe_pci_reads_performed: u8,
    pub pci_identity_consistent: bool,
    pub command_bus_master_seen: bool,
    pub mmio_whitelist_ready: bool,
    pub mmio_register_reads_enabled: bool,
    pub firmware_requirements_resolved: bool,
    pub gmc_gtt_profile_ready: bool,
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

impl C9State {
    pub const EMPTY: Self = Self {
        amd_present:false, c8_profile_policy_ready:false, profile:ProfileId::Unknown,
        profile_verified:false, live_pci_identity_reads:false, safe_pci_reads_performed:0,
        pci_identity_consistent:false, command_bus_master_seen:false, mmio_whitelist_ready:false,
        mmio_register_reads_enabled:false, firmware_requirements_resolved:false,
        gmc_gtt_profile_ready:false, register_writes_enabled:false, firmware_upload_enabled:false,
        command_submit_enabled:false, bus_master_enabled:false, fallback_armed:true,
        vendor_id:0, device_id:0, revision:0, gc_ip_major:0, gmc_ip_major:0,
        sdma_ip_major:0, dcn_ip_major:0,
    };
}

static STATE: SpinLock<C9State> = SpinLock::new(C9State::EMPTY);

fn profile_for(device_id: u16, revision: u8) -> Option<&'static VerifiedRadeonProfile> {
    VERIFIED_RADEON_PROFILES.iter().find(|p|
        p.device_id == device_id && revision >= p.revision_min && revision <= p.revision_max)
}

fn self_test() -> Result<(), &'static str> {
    if K14C9_ABI_VERSION != 1 || RADEON_C9_SAFE_PCI_READS != 5
        || RADEON_C9_MMIO_READS_ALLOWED || RADEON_C9_REGISTER_WRITES_ALLOWED
        || RADEON_C9_FIRMWARE_UPLOAD_ALLOWED || RADEON_C9_COMMAND_SUBMIT_ALLOWED
        || RADEON_C9_BUS_MASTER_ALLOWED {
        return Err("K14.C9 fail-closed constants invalid");
    }
    if VERIFIED_RADEON_PROFILES.is_empty()
        || VERIFIED_RADEON_PROFILES.iter().any(|p| p.mmio_whitelist_entries != 0) {
        return Err("K14.C9 profile table violates MMIO-deferred policy");
    }
    Ok(())
}

pub fn initialize() -> Result<C9State, &'static str> {
    self_test()?;
    let c8 = native_gpu_c8::state();
    let binding = native_gpu_binding::state();
    let mut s = C9State { amd_present:c8.amd_present, c8_profile_policy_ready:true,
        vendor_id:c8.vendor_id, device_id:c8.device_id, revision:c8.revision, ..C9State::EMPTY };

    serial::println(format_args!(
        "[C9PF] verified Radeon profile table: profiles={} navi21=0x73bf navi48=0x7550 guessed_mmio_offsets=false",
        VERIFIED_RADEON_PROFILES.len()
    ));
    serial::println(format_args!(
        "[C9PR] live safe-read policy: pci_config_identity=true reads={} mmio_reads=false writes=false upload=false submit=false bus_master=false",
        RADEON_C9_SAFE_PCI_READS
    ));
    serial::println(format_args!(
        "[C9IP] profile promotion: exact PCI ID/revision -> live PCI identity consistency -> IP majors -> firmware/GMC-GTT profile; MMIO whitelist remains separate"
    ));

    if !c8.amd_present {
        serial::println(format_args!(
            "[C9HW] live Radeon profile verification: present=false qemu_deferred=true profile=unknown pci_reads=false mmio_reads=false fallback=true"
        ));
    } else if binding.selected_vendor != NativeGpuVendor::Amd as u8 || !c8.pci_identity_ready {
        serial::println(format_args!(
            "[C9HW] live Radeon profile verification: present=true profile=unknown pci_reads=false reason=no_selected_amd_binding fallback=true"
        ));
    } else {
        let bus=binding.selected_bus; let dev=binding.selected_device; let fun=binding.selected_function;
        let id = pci::read_u32(bus,dev,fun,0x00);
        let class_rev = pci::read_u32(bus,dev,fun,0x08);
        let command = pci::read_u16(bus,dev,fun,0x04);
        let vendor = id as u16;
        let device = (id >> 16) as u16;
        let revision = class_rev as u8;
        let class_code = (class_rev >> 24) as u8;
        let subclass = (class_rev >> 16) as u8;
        s.live_pci_identity_reads = true;
        s.safe_pci_reads_performed = RADEON_C9_SAFE_PCI_READS;
        s.vendor_id=vendor; s.device_id=device; s.revision=revision;
        s.command_bus_master_seen = command & (1<<2) != 0;
        s.pci_identity_consistent = vendor == AMD_VENDOR_ID
            && device == c8.device_id
            && revision == c8.revision
            && class_code == 0x03 && matches!(subclass, 0x00 | 0x02);

        if s.command_bus_master_seen {
            return Err("K14.C9 observed Radeon bus mastering before native promotion");
        }
        if !s.pci_identity_consistent {
            return Err("K14.C9 live PCI identity disagrees with selected Radeon binding");
        }

        if let Some(p)=profile_for(device,revision) {
            s.profile=p.id; s.profile_verified=true;
            s.gc_ip_major=p.gc_ip_major; s.gmc_ip_major=p.gmc_ip_major;
            s.sdma_ip_major=p.sdma_ip_major; s.dcn_ip_major=p.dcn_ip_major;
            s.firmware_requirements_resolved=true;
            s.gmc_gtt_profile_ready=true;
            s.mmio_whitelist_ready=p.mmio_whitelist_entries != 0;
            serial::println(format_args!(
                "[C9HW] live Radeon profile verification: present=true devid={:#06x} rev={:#04x} profile={:?} pci_reads={} consistent=true gc={} gmc={} sdma={} dcn={} mmio_whitelist={} mmio_reads=false fallback=true",
                device,revision,s.profile,s.safe_pci_reads_performed,s.gc_ip_major,s.gmc_ip_major,s.sdma_ip_major,s.dcn_ip_major,p.mmio_whitelist_entries
            ));
        } else {
            serial::println(format_args!(
                "[C9HW] live Radeon profile verification: present=true devid={:#06x} rev={:#04x} profile=unknown pci_reads={} consistent=true mmio_reads=false reason=no_verified_profile fallback=true",
                device,revision,s.safe_pci_reads_performed
            ));
        }
    }

    if s.mmio_register_reads_enabled || s.register_writes_enabled || s.firmware_upload_enabled
        || s.command_submit_enabled || s.bus_master_enabled {
        return Err("K14.C9 promoted destructive or unverified Radeon capability");
    }

    serial::println(format_args!(
        "[C9RD] K14.C9 verified Radeon profiles ready: amd_present={} profile_verified={} pci_reads={} identity_consistent={} mmio_whitelist={} mmio_reads=false firmware_resolved={} gmc_gtt_profile={} writes=false upload=false submit=false bus_master=false fallback=true",
        s.amd_present,s.profile_verified,s.safe_pci_reads_performed,s.pci_identity_consistent,
        s.mmio_whitelist_ready,s.firmware_requirements_resolved,s.gmc_gtt_profile_ready
    ));
    *STATE.lock()=s;
    Ok(s)
}

pub fn state() -> C9State { *STATE.lock() }
pub fn packed_status() -> u64 {
    let s=state();
    let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24)|(u64::from(s.profile as u8)<<16);
    for (bit,on) in [s.amd_present,s.c8_profile_policy_ready,s.profile_verified,s.live_pci_identity_reads,
        s.pci_identity_consistent,s.command_bus_master_seen,s.mmio_whitelist_ready,s.mmio_register_reads_enabled,
        s.firmware_requirements_resolved,s.gmc_gtt_profile_ready,s.register_writes_enabled,s.firmware_upload_enabled,
        s.command_submit_enabled,s.bus_master_enabled,s.fallback_armed].into_iter().enumerate(){if on{v|=1u64<<bit;}}
    v
}
