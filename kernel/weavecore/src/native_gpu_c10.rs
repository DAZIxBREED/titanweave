//! K14.C10 per-IP MMIO whitelist engine + guarded live-read activation.
//!
//! C10 takes C9's verified Radeon device profiles and introduces the actual
//! execution machinery for bounded, side-effect-free MMIO reads.  The engine
//! is deliberately fail-closed: a physical Radeon may only be read when C7's
//! supervisor-only read-only aperture is live, C9's PCI identity is consistent,
//! and the exact profile has a non-empty reviewed whitelist.  This first C10
//! cut does not invent register offsets; hardware whitelists remain empty until
//! an offset has been reviewed against AMD/public register definitions for the
//! exact IP revision.  No MMIO write, firmware upload, command submission, or
//! Radeon bus mastering is enabled.

use crate::{native_gpu_c7, native_gpu_c9, serial, sync::SpinLock};

pub const K14C10_ABI_VERSION: u32 = 1;
pub const RADEON_C10_MAX_MMIO_READS: u8 = 16;
pub const RADEON_C10_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C10_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C10_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C10_BUS_MASTER_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum IpBlock { Gc=1, Gmc=2, Sdma=3, Dcn=4 }

#[derive(Clone, Copy, Debug)]
pub struct MmioReadDescriptor {
    pub ip: IpBlock,
    pub offset: u32,
    pub width: u8,
    pub side_effect_free: bool,
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct ProfileMmioWhitelist {
    pub profile: native_gpu_c9::ProfileId,
    pub entries: &'static [MmioReadDescriptor],
}

// C10 intentionally ships no physical offsets until each offset is reviewed
// for the exact IP revision.  The engine below is live and bounded; the lack of
// entries therefore keeps real MMIO reads fenced rather than guessing.
static NAVI21_MMIO_READS: &[MmioReadDescriptor] = &[];
static NAVI48_MMIO_READS: &[MmioReadDescriptor] = &[];
static VERIFIED_MMIO_WHITELISTS: &[ProfileMmioWhitelist] = &[
    ProfileMmioWhitelist { profile:native_gpu_c9::ProfileId::Navi21Rx6800_6900, entries:NAVI21_MMIO_READS },
    ProfileMmioWhitelist { profile:native_gpu_c9::ProfileId::Navi48Rx9070, entries:NAVI48_MMIO_READS },
];

#[derive(Clone, Copy, Debug)]
pub struct C10State {
    pub amd_present: bool,
    pub profile_verified: bool,
    pub pci_identity_consistent: bool,
    pub read_only_aperture_ready: bool,
    pub whitelist_profile_found: bool,
    pub whitelist_reviewed: bool,
    pub whitelist_entries: u8,
    pub live_mmio_reads_enabled: bool,
    pub live_mmio_reads_performed: u8,
    pub bounds_checks_passed: bool,
    pub write_path_fenced: bool,
    pub firmware_upload_enabled: bool,
    pub command_submit_enabled: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub device_id: u16,
    pub revision: u8,
}
impl C10State { pub const EMPTY: Self = Self {
    amd_present:false, profile_verified:false, pci_identity_consistent:false,
    read_only_aperture_ready:false, whitelist_profile_found:false, whitelist_reviewed:false,
    whitelist_entries:0, live_mmio_reads_enabled:false, live_mmio_reads_performed:0,
    bounds_checks_passed:false, write_path_fenced:true, firmware_upload_enabled:false,
    command_submit_enabled:false, bus_master_enabled:false, fallback_armed:true,
    device_id:0, revision:0,
}; }
static STATE: SpinLock<C10State> = SpinLock::new(C10State::EMPTY);

fn whitelist_for(profile:native_gpu_c9::ProfileId) -> Option<&'static ProfileMmioWhitelist> {
    VERIFIED_MMIO_WHITELISTS.iter().find(|w| w.profile==profile)
}
fn descriptor_valid(d:&MmioReadDescriptor, aperture_bytes:u64) -> bool {
    matches!(d.width, 1|2|4|8) && d.side_effect_free
        && (d.offset as u64).checked_add(d.width as u64).is_some_and(|end| end <= aperture_bytes)
        && (d.offset as u64) % (d.width as u64) == 0
}
unsafe fn read_descriptor(base:u64, d:&MmioReadDescriptor) -> u64 {
    let p=(base+d.offset as u64) as *const u8;
    match d.width {
        1 => unsafe { core::ptr::read_volatile(p) as u64 },
        2 => unsafe { core::ptr::read_volatile(p.cast::<u16>()) as u64 },
        4 => unsafe { core::ptr::read_volatile(p.cast::<u32>()) as u64 },
        8 => unsafe { core::ptr::read_volatile(p.cast::<u64>()) },
        _ => 0,
    }
}
fn self_test() -> Result<(), &'static str> {
    if K14C10_ABI_VERSION!=1 || RADEON_C10_MAX_MMIO_READS==0
        || RADEON_C10_MMIO_WRITES_ALLOWED || RADEON_C10_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C10_COMMAND_SUBMIT_ALLOWED || RADEON_C10_BUS_MASTER_ALLOWED {
        return Err("K14.C10 fail-closed constants invalid");
    }
    let good=MmioReadDescriptor{ip:IpBlock::Gc,offset:0,width:4,side_effect_free:true,name:"selftest"};
    let bad=MmioReadDescriptor{ip:IpBlock::Gc,offset:4094,width:4,side_effect_free:true,name:"oob"};
    if !descriptor_valid(&good,4096) || descriptor_valid(&bad,4096) { return Err("K14.C10 bounds validator failed"); }
    Ok(())
}

pub fn initialize() -> Result<C10State, &'static str> {
    self_test()?;
    let c7=native_gpu_c7::state();
    let c9=native_gpu_c9::state();
    let mut s=C10State { amd_present:c9.amd_present, profile_verified:c9.profile_verified,
        pci_identity_consistent:c9.pci_identity_consistent, read_only_aperture_ready:c7.read_only_mmio_mapped,
        device_id:c9.device_id, revision:c9.revision, bounds_checks_passed:true, ..C10State::EMPTY };

    serial::println(format_args!(
        "[C10WL] per-IP MMIO whitelist engine: max_reads={} bounds_checked=true aligned=true side_effect_free_only=true unknown_offset=fenced",
        RADEON_C10_MAX_MMIO_READS));
    serial::println(format_args!(
        "[C10RD] live-read activation policy: c7_readonly_aperture -> c9_verified_profile -> reviewed_nonempty_whitelist -> bounded_volatile_reads; writes=false upload=false submit=false bus_master=false"));

    if !c9.amd_present {
        serial::println(format_args!(
            "[C10HW] live Radeon MMIO reads: present=false qemu_deferred=true whitelist=false reads=false writes=false fallback=true"));
    } else if !c9.profile_verified || !c9.pci_identity_consistent || !c7.read_only_mmio_mapped {
        serial::println(format_args!(
            "[C10HW] live Radeon MMIO reads: present=true devid={:#06x} profile_verified={} identity_consistent={} ro_mmio={} reads=false reason=prerequisite_not_live fallback=true",
            c9.device_id,c9.profile_verified,c9.pci_identity_consistent,c7.read_only_mmio_mapped));
    } else if let Some(w)=whitelist_for(c9.profile) {
        s.whitelist_profile_found=true;
        s.whitelist_entries=w.entries.len().min(u8::MAX as usize) as u8;
        s.whitelist_reviewed=!w.entries.is_empty()
            && w.entries.len() <= RADEON_C10_MAX_MMIO_READS as usize
            && w.entries.iter().all(|d| descriptor_valid(d, native_gpu_c7::RADEON_C7_PROBE_BYTES));
        if s.whitelist_reviewed {
            // Reads are possible only through C7's read-only mapping.  The
            // current reviewed tables are intentionally empty, so this block
            // is unreachable until a later source revision adds exact offsets.
            for d in w.entries {
                let _value=unsafe { read_descriptor(c7.read_only_mmio_virt,d) };
                s.live_mmio_reads_performed=s.live_mmio_reads_performed.saturating_add(1);
                serial::println(format_args!("[C10R ] safe MMIO read: ip={:?} name={} offset={:#x} width={}",d.ip,d.name,d.offset,d.width));
            }
            s.live_mmio_reads_enabled=s.live_mmio_reads_performed>0;
        }
        serial::println(format_args!(
            "[C10HW] live Radeon MMIO reads: present=true devid={:#06x} profile={:?} whitelist_profile=true entries={} reviewed={} reads={} performed={} writes=false fallback=true",
            c9.device_id,c9.profile,s.whitelist_entries,s.whitelist_reviewed,s.live_mmio_reads_enabled,s.live_mmio_reads_performed));
    } else {
        serial::println(format_args!(
            "[C10HW] live Radeon MMIO reads: present=true devid={:#06x} whitelist_profile=false reads=false reason=no_profile_whitelist fallback=true",c9.device_id));
    }

    if s.live_mmio_reads_enabled && (!s.read_only_aperture_ready || !s.profile_verified || !s.pci_identity_consistent || !s.whitelist_reviewed) {
        return Err("K14.C10 enabled MMIO reads without all safety gates");
    }
    if !s.write_path_fenced || s.firmware_upload_enabled || s.command_submit_enabled || s.bus_master_enabled {
        return Err("K14.C10 destructive Radeon capability promoted early");
    }
    serial::println(format_args!(
        "[C10NF] K14.C10 per-IP MMIO whitelist engine ready: amd_present={} profile_verified={} ro_mmio={} whitelist_profile={} whitelist_reviewed={} entries={} live_reads={} performed={} writes=false upload=false submit=false bus_master=false fallback=true",
        s.amd_present,s.profile_verified,s.read_only_aperture_ready,s.whitelist_profile_found,s.whitelist_reviewed,
        s.whitelist_entries,s.live_mmio_reads_enabled,s.live_mmio_reads_performed));
    *STATE.lock()=s; Ok(s)
}
pub fn state()->C10State{*STATE.lock()}
pub fn packed_status()->u64{
    let s=state(); let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24)|(u64::from(s.whitelist_entries)<<16);
    for (bit,on) in [s.amd_present,s.profile_verified,s.pci_identity_consistent,s.read_only_aperture_ready,
        s.whitelist_profile_found,s.whitelist_reviewed,s.live_mmio_reads_enabled,s.bounds_checks_passed,
        s.write_path_fenced,s.firmware_upload_enabled,s.command_submit_enabled,s.bus_master_enabled,s.fallback_armed]
        .into_iter().enumerate(){if on{v|=1u64<<bit;}} v
}
