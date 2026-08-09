//! K14.C11 reviewed Radeon register definitions + IP-base resolver gate.
//!
//! C10 proved the bounded read executor but intentionally carried no physical
//! offsets. C11 introduces the first reviewed register definitions for the
//! verified Navi 21 (GC 10.3 / SDMA 5.2) and Navi 48 (GC 12.0 / SDMA 7.x)
//! profiles. AMD's generated register headers describe these as IP-relative
//! register indices; they are NOT safe to treat as BAR-relative byte offsets.
//! Therefore C11 also adds an explicit IP-base resolver gate. Until a trusted
//! IP-discovery base map is available, the reviewed definitions are present but
//! physical MMIO dereferences remain fenced.
//!
//! Source provenance used for the reviewed definitions:
//! - Linux/AMD generated gc_10_3_0_offset.h
//! - Linux/AMD generated gc_12_0_0_offset.h
//! These headers are AMD-authored register definitions carried by upstream
//! Linux. C11 does not copy masks or write-side programming sequences.

use crate::{native_gpu_c10, native_gpu_c9, serial, sync::SpinLock};

pub const K14C11_ABI_VERSION: u32 = 1;
pub const RADEON_C11_MAX_REVIEWED_REGISTERS: u8 = 8;
pub const RADEON_C11_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C11_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C11_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C11_BUS_MASTER_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RegisterIp { Gc=1, Sdma=2 }

#[derive(Clone, Copy, Debug)]
pub struct ReviewedRegister {
    pub ip: RegisterIp,
    pub register_index: u32,
    pub width: u8,
    pub side_effect_free: bool,
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct ReviewedProfileRegisters {
    pub profile: native_gpu_c9::ProfileId,
    pub entries: &'static [ReviewedRegister],
}

// AMD gc_10_3_0_offset.h:
//   mmGRBM_STATUS          0x0da4
//   mmGRBM_CHIP_REVISION   0x0dc1
//   mmSDMA0_STATUS_REG     0x0025
static NAVI21_REVIEWED: &[ReviewedRegister] = &[
    ReviewedRegister { ip:RegisterIp::Gc,   register_index:0x0da4, width:4, side_effect_free:true, name:"GRBM_STATUS" },
    ReviewedRegister { ip:RegisterIp::Gc,   register_index:0x0dc1, width:4, side_effect_free:true, name:"GRBM_CHIP_REVISION" },
    ReviewedRegister { ip:RegisterIp::Sdma, register_index:0x0025, width:4, side_effect_free:true, name:"SDMA0_STATUS_REG" },
];

// AMD gc_12_0_0_offset.h:
//   regGRBM_STATUS         0x0da4
//   regGRBM_CHIP_REVISION  0x0dc1
//   regSDMA0_STATUS_REG    0x0024
static NAVI48_REVIEWED: &[ReviewedRegister] = &[
    ReviewedRegister { ip:RegisterIp::Gc,   register_index:0x0da4, width:4, side_effect_free:true, name:"GRBM_STATUS" },
    ReviewedRegister { ip:RegisterIp::Gc,   register_index:0x0dc1, width:4, side_effect_free:true, name:"GRBM_CHIP_REVISION" },
    ReviewedRegister { ip:RegisterIp::Sdma, register_index:0x0024, width:4, side_effect_free:true, name:"SDMA0_STATUS_REG" },
];

static REVIEWED_REGISTER_PROFILES: &[ReviewedProfileRegisters] = &[
    ReviewedProfileRegisters { profile:native_gpu_c9::ProfileId::Navi21Rx6800_6900, entries:NAVI21_REVIEWED },
    ReviewedProfileRegisters { profile:native_gpu_c9::ProfileId::Navi48Rx9070, entries:NAVI48_REVIEWED },
];

#[derive(Clone, Copy, Debug)]
pub struct IpBaseMap {
    /// GC base in DWORD register-index units, as resolved from trusted IP discovery.
    pub gc_base_dwords: u64,
    /// SDMA0 base in DWORD register-index units, as resolved from trusted IP discovery.
    pub sdma0_base_dwords: u64,
    pub gc_valid: bool,
    pub sdma0_valid: bool,
}
impl IpBaseMap { pub const EMPTY: Self=Self{gc_base_dwords:0,sdma0_base_dwords:0,gc_valid:false,sdma0_valid:false}; }

#[derive(Clone, Copy, Debug)]
pub struct C11State {
    pub amd_present: bool,
    pub profile_verified: bool,
    pub reviewed_profile_found: bool,
    pub reviewed_entries: u8,
    pub register_definitions_reviewed: bool,
    pub ip_base_map_ready: bool,
    pub address_translation_verified: bool,
    pub live_mmio_reads_enabled: bool,
    pub live_mmio_reads_performed: u8,
    pub write_path_fenced: bool,
    pub firmware_upload_enabled: bool,
    pub command_submit_enabled: bool,
    pub bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub device_id: u16,
    pub revision: u8,
}
impl C11State { pub const EMPTY:Self=Self{
    amd_present:false,profile_verified:false,reviewed_profile_found:false,reviewed_entries:0,
    register_definitions_reviewed:false,ip_base_map_ready:false,address_translation_verified:false,
    live_mmio_reads_enabled:false,live_mmio_reads_performed:0,write_path_fenced:true,
    firmware_upload_enabled:false,command_submit_enabled:false,bus_master_enabled:false,fallback_armed:true,
    device_id:0,revision:0,
}; }
static STATE:SpinLock<C11State>=SpinLock::new(C11State::EMPTY);

fn registers_for(profile:native_gpu_c9::ProfileId)->Option<&'static ReviewedProfileRegisters>{
    REVIEWED_REGISTER_PROFILES.iter().find(|p|p.profile==profile)
}

fn reviewed_definition_valid(r:&ReviewedRegister)->bool{
    r.width==4 && r.side_effect_free && r.register_index<=0x00ff_ffff
}

/// Resolve an AMD IP-relative DWORD register index to a byte offset inside a
/// CPU-visible MMIO aperture. This function intentionally requires a trusted
/// per-IP base map; callers cannot silently interpret a register index as a BAR
/// byte offset.
fn resolve_byte_offset(map:IpBaseMap,r:&ReviewedRegister)->Option<u64>{
    let base=match r.ip {
        RegisterIp::Gc if map.gc_valid => map.gc_base_dwords,
        RegisterIp::Sdma if map.sdma0_valid => map.sdma0_base_dwords,
        _ => return None,
    };
    base.checked_add(r.register_index as u64)?.checked_mul(4)
}

fn self_test()->Result<(), &'static str>{
    if K14C11_ABI_VERSION!=1 || RADEON_C11_MAX_REVIEWED_REGISTERS<3
        || RADEON_C11_MMIO_WRITES_ALLOWED || RADEON_C11_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C11_COMMAND_SUBMIT_ALLOWED || RADEON_C11_BUS_MASTER_ALLOWED {
        return Err("K14.C11 fail-closed constants invalid");
    }
    if REVIEWED_REGISTER_PROFILES.len()!=2
        || REVIEWED_REGISTER_PROFILES.iter().any(|p|p.entries.is_empty() || p.entries.len()>RADEON_C11_MAX_REVIEWED_REGISTERS as usize
            || p.entries.iter().any(|r|!reviewed_definition_valid(r))) {
        return Err("K14.C11 reviewed register table invalid");
    }
    let map=IpBaseMap{gc_base_dwords:0x1000,sdma0_base_dwords:0x2000,gc_valid:true,sdma0_valid:true};
    let gc=resolve_byte_offset(map,&NAVI21_REVIEWED[0]).ok_or("K14.C11 resolver self-test failed")?;
    if gc != (0x1000u64+0x0da4)*4 { return Err("K14.C11 resolver produced wrong byte offset"); }
    if resolve_byte_offset(IpBaseMap::EMPTY,&NAVI21_REVIEWED[0]).is_some() { return Err("K14.C11 resolver accepted missing IP base"); }
    Ok(())
}

pub fn initialize()->Result<C11State,&'static str>{
    self_test()?;
    let c9=native_gpu_c9::state();
    let c10=native_gpu_c10::state();
    let mut s=C11State{amd_present:c9.amd_present,profile_verified:c9.profile_verified,
        device_id:c9.device_id,revision:c9.revision,..C11State::EMPTY};

    serial::println(format_args!(
        "[C11RF] reviewed Radeon register definitions: profiles={} navi21_entries={} navi48_entries={} source=AMD_upstream_generated_headers writes=false",
        REVIEWED_REGISTER_PROFILES.len(),NAVI21_REVIEWED.len(),NAVI48_REVIEWED.len()));
    serial::println(format_args!(
        "[C11BA] IP-base address resolver: register_unit=dword require_trusted_ip_base=true bar_plus_raw_index=false bounds_before_read=true"));

    if let Some(p)=registers_for(c9.profile) {
        s.reviewed_profile_found=true;
        s.reviewed_entries=p.entries.len().min(u8::MAX as usize) as u8;
        s.register_definitions_reviewed=p.entries.iter().all(reviewed_definition_valid);
    }

    // C11 intentionally does not fabricate GC/SDMA base addresses. The base
    // map must come from a later trusted AMD IP-discovery parser. Until then,
    // even a real matching Radeon remains read-fenced.
    let ip_bases=IpBaseMap::EMPTY;
    s.ip_base_map_ready=ip_bases.gc_valid && ip_bases.sdma0_valid;
    s.address_translation_verified=s.ip_base_map_ready && s.reviewed_profile_found;

    if !c9.amd_present {
        serial::println(format_args!(
            "[C11HW] physical Radeon register reads: present=false qemu_deferred=true reviewed_defs=true ip_bases=false reads=false writes=false fallback=true"));
    } else if !c9.profile_verified || !c9.pci_identity_consistent || !c10.read_only_aperture_ready {
        serial::println(format_args!(
            "[C11HW] physical Radeon register reads: present=true devid={:#06x} profile_verified={} identity={} ro_mmio={} reads=false reason=prerequisite_not_live fallback=true",
            c9.device_id,c9.profile_verified,c9.pci_identity_consistent,c10.read_only_aperture_ready));
    } else if !s.register_definitions_reviewed {
        serial::println(format_args!(
            "[C11HW] physical Radeon register reads: present=true devid={:#06x} profile={:?} reads=false reason=no_reviewed_register_set fallback=true",
            c9.device_id,c9.profile));
    } else if !s.ip_base_map_ready {
        serial::println(format_args!(
            "[C11HW] physical Radeon register reads: present=true devid={:#06x} profile={:?} reviewed_entries={} ip_bases=false reads=false reason=ip_base_map_unresolved fallback=true",
            c9.device_id,c9.profile,s.reviewed_entries));
    }

    if s.live_mmio_reads_enabled || s.live_mmio_reads_performed!=0 {
        return Err("K14.C11 performed MMIO read before trusted IP-base resolution");
    }
    if !s.write_path_fenced || s.firmware_upload_enabled || s.command_submit_enabled || s.bus_master_enabled {
        return Err("K14.C11 destructive Radeon capability promoted early");
    }

    serial::println(format_args!(
        "[C11RD] K14.C11 reviewed register whitelist ready: amd_present={} profile_verified={} reviewed_profile={} entries={} definitions_reviewed={} ip_bases={} address_translation={} live_reads={} performed={} writes=false upload=false submit=false bus_master=false fallback=true",
        s.amd_present,s.profile_verified,s.reviewed_profile_found,s.reviewed_entries,s.register_definitions_reviewed,
        s.ip_base_map_ready,s.address_translation_verified,s.live_mmio_reads_enabled,s.live_mmio_reads_performed));
    *STATE.lock()=s; Ok(s)
}

pub fn state()->C11State{*STATE.lock()}
pub fn packed_status()->u64{
    let s=state(); let mut v=(u64::from(s.device_id)<<32)|(u64::from(s.revision)<<24)|(u64::from(s.reviewed_entries)<<16);
    for (bit,on) in [s.amd_present,s.profile_verified,s.reviewed_profile_found,s.register_definitions_reviewed,
        s.ip_base_map_ready,s.address_translation_verified,s.live_mmio_reads_enabled,s.write_path_fenced,
        s.firmware_upload_enabled,s.command_submit_enabled,s.bus_master_enabled,s.fallback_armed].into_iter().enumerate(){
        if on{v|=1u64<<bit;}
    } v
}
