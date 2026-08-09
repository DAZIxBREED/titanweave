//! K14.C20 checksum-qualified AMD IP-record enumeration and exact base resolver.
//!
//! C19 can acquire and checksum-qualify the physical AMD IP-discovery snapshot.
//! C20 consumes only that verified snapshot and walks AMD's packed die/IP records
//! with strict bounds.  It imports the source-backed hardware IDs for GC and the
//! four SDMA IDs, handles both 32-bit and v4 64-bit base-address encodings, and
//! records the exact instance-0 dword bases supplied by the GPU.
//!
//! C20 does NOT touch Radeon MMIO, does NOT mutate the frozen C12/C16 state, and
//! does NOT submit work.  It merely produces a trusted live base map that a later
//! milestone may consume to promote the reviewed C16 write target.

use crate::{native_gpu_c9, native_gpu_c17, native_gpu_c19, serial, sync::SpinLock};

pub const K14C20_ABI_VERSION: u32 = 1;

// Source-backed AMD HW IDs from drivers/gpu/drm/amd/include/soc15_hw_ip.h.
pub const AMD_GC_HWID: u16 = 11;
pub const AMD_SDMA0_HWID: u16 = 42;
pub const AMD_SDMA1_HWID: u16 = 43;
pub const AMD_SDMA2_HWID: u16 = 68;
pub const AMD_SDMA3_HWID: u16 = 69;

pub const RADEON_C20_MAX_IPS_PER_DIE: u16 = 256;
pub const RADEON_C20_MAX_TOTAL_IPS: u16 = 1024;
pub const RADEON_C20_FIXED_IP_BYTES: usize = 8;
pub const RADEON_C20_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C20_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C20_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C20_BUS_MASTER_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedIp {
    pub found: bool,
    pub hw_id: u16,
    pub instance: u8,
    pub major: u8,
    pub minor: u8,
    pub revision: u8,
    pub sub_revision: u8,
    pub variant: u8,
    pub num_base_addresses: u8,
    pub base0_dwords: u64,
}
impl ResolvedIp {
    pub const EMPTY: Self = Self {
        found: false,
        hw_id: 0,
        instance: 0,
        major: 0,
        minor: 0,
        revision: 0,
        sub_revision: 0,
        variant: 0,
        num_base_addresses: 0,
        base0_dwords: 0,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct Resolution {
    pub valid: bool,
    pub ip_version: u16,
    pub num_dies: u16,
    pub records_scanned: u16,
    pub base_addr_64_bit: bool,
    pub gc: ResolvedIp,
    pub sdma0: ResolvedIp,
    pub sdma1: ResolvedIp,
    pub sdma2: ResolvedIp,
    pub sdma3: ResolvedIp,
}
impl Resolution {
    pub const EMPTY: Self = Self {
        valid: false,
        ip_version: 0,
        num_dies: 0,
        records_scanned: 0,
        base_addr_64_bit: false,
        gc: ResolvedIp::EMPTY,
        sdma0: ResolvedIp::EMPTY,
        sdma1: ResolvedIp::EMPTY,
        sdma2: ResolvedIp::EMPTY,
        sdma3: ResolvedIp::EMPTY,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct C20State {
    pub amd_present: bool,
    pub navi48: bool,
    pub c19_snapshot_verified: bool,
    pub parser_ready: bool,
    pub source_hwids_imported: bool,
    pub records_scanned: u16,
    pub ip_version: u16,
    pub num_dies: u16,
    pub base_addr_64_bit: bool,
    pub gc_resolved: bool,
    pub sdma0_resolved: bool,
    pub sdma1_resolved: bool,
    pub sdma2_resolved: bool,
    pub sdma3_resolved: bool,
    pub gc_base_dwords: u64,
    pub sdma0_base_dwords: u64,
    pub sdma1_base_dwords: u64,
    pub sdma2_base_dwords: u64,
    pub sdma3_base_dwords: u64,
    pub gc_major: u8,
    pub gc_minor: u8,
    pub gc_revision: u8,
    pub exact_base_set_ready: bool,
    pub c16_promotion_input_ready: bool,
    pub snapshot_fingerprint: u64,
    pub mmio_write_enabled: bool,
    pub firmware_upload_enabled: bool,
    pub command_submit_enabled: bool,
    pub radeon_bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub device_id: u16,
    pub revision: u8,
}
impl C20State {
    pub const EMPTY: Self = Self {
        amd_present: false,
        navi48: false,
        c19_snapshot_verified: false,
        parser_ready: false,
        source_hwids_imported: false,
        records_scanned: 0,
        ip_version: 0,
        num_dies: 0,
        base_addr_64_bit: false,
        gc_resolved: false,
        sdma0_resolved: false,
        sdma1_resolved: false,
        sdma2_resolved: false,
        sdma3_resolved: false,
        gc_base_dwords: 0,
        sdma0_base_dwords: 0,
        sdma1_base_dwords: 0,
        sdma2_base_dwords: 0,
        sdma3_base_dwords: 0,
        gc_major: 0,
        gc_minor: 0,
        gc_revision: 0,
        exact_base_set_ready: false,
        c16_promotion_input_ready: false,
        snapshot_fingerprint: 0,
        mmio_write_enabled: false,
        firmware_upload_enabled: false,
        command_submit_enabled: false,
        radeon_bus_master_enabled: false,
        fallback_armed: true,
        device_id: 0,
        revision: 0,
    };
}

static STATE: SpinLock<C20State> = SpinLock::new(C20State::EMPTY);

fn le16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(o)?, *b.get(o + 1)?]))
}
fn le32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *b.get(o)?, *b.get(o + 1)?, *b.get(o + 2)?, *b.get(o + 3)?,
    ]))
}
fn le64(b: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes([
        *b.get(o)?, *b.get(o + 1)?, *b.get(o + 2)?, *b.get(o + 3)?,
        *b.get(o + 4)?, *b.get(o + 5)?, *b.get(o + 6)?, *b.get(o + 7)?,
    ]))
}

fn set_unique(slot: &mut ResolvedIp, candidate: ResolvedIp) -> Result<(), &'static str> {
    if !candidate.found || candidate.instance != 0 || candidate.base0_dwords == 0 {
        return Ok(());
    }
    if !slot.found {
        *slot = candidate;
        return Ok(());
    }
    if slot.hw_id != candidate.hw_id
        || slot.instance != candidate.instance
        || slot.major != candidate.major
        || slot.minor != candidate.minor
        || slot.revision != candidate.revision
        || slot.base0_dwords != candidate.base0_dwords
    {
        return Err("K14.C20 conflicting duplicate AMD IP-discovery target record");
    }
    Ok(())
}

/// Enumerate AMD's packed IP records using the same source layout used by
/// amdgpu_discovery_reg_base_init().  Die offsets are absolute offsets in the
/// discovery binary.  Each IP record has an 8-byte fixed prefix followed by
/// num_base_address entries of either 32 or 64 bits.  For v4 64-bit entries,
/// AMDGPU stores the low 32 bits and clears the top two dword bits because the
/// register base is expressed in dwords.
pub fn resolve_verified_snapshot(b: &[u8]) -> Result<Resolution, &'static str> {
    let top = native_gpu_c17::parse_discovery_snapshot(b)?;
    if !top.valid {
        return Err("K14.C20 C17 parser did not validate discovery snapshot");
    }
    if top.binary_size as usize > b.len() {
        return Err("K14.C20 binary size exceeds snapshot");
    }
    let ip_off = top.ip_table_offset as usize;
    if top.ip_table_size < 80 {
        return Err("K14.C20 AMD IP-discovery header shorter than packed die table");
    }
    let binary_end = top.binary_size as usize;
    let ip_end = ip_off
        .checked_add(top.ip_table_size as usize)
        .ok_or("K14.C20 IP table end overflow")?;
    if ip_end > binary_end {
        return Err("K14.C20 IP table exceeds binary");
    }

    let mut out = Resolution {
        valid: true,
        ip_version: top.ip_version,
        num_dies: top.num_dies,
        base_addr_64_bit: top.base_addr_64_bit,
        ..Resolution::EMPTY
    };

    for die_index in 0..top.num_dies {
        let info = ip_off
            .checked_add(14)
            .and_then(|v| v.checked_add(die_index as usize * 4))
            .ok_or("K14.C20 die-info offset overflow")?;
        let listed_die_id = le16(b, info).ok_or("K14.C20 truncated die-info id")?;
        let die_offset = le16(b, info + 2).ok_or("K14.C20 truncated die-info offset")? as usize;
        if listed_die_id != die_index {
            return Err("K14.C20 die-info id does not match bounded die index");
        }
        if die_offset < ip_off || die_offset.checked_add(4).ok_or("K14.C20 die header overflow")? > binary_end {
            return Err("K14.C20 die header outside discovery binary");
        }
        let die_id = le16(b, die_offset).ok_or("K14.C20 truncated die header")?;
        let num_ips = le16(b, die_offset + 2).ok_or("K14.C20 truncated IP count")?;
        if die_id != die_index {
            return Err("K14.C20 die header id mismatch");
        }
        if num_ips > RADEON_C20_MAX_IPS_PER_DIE {
            return Err("K14.C20 per-die IP count exceeds bounded policy");
        }

        let mut cursor = die_offset + 4;
        for _ in 0..num_ips {
            out.records_scanned = out.records_scanned.checked_add(1)
                .ok_or("K14.C20 total IP count overflow")?;
            if out.records_scanned > RADEON_C20_MAX_TOTAL_IPS {
                return Err("K14.C20 total IP count exceeds bounded policy");
            }
            if cursor.checked_add(RADEON_C20_FIXED_IP_BYTES).ok_or("K14.C20 IP prefix overflow")? > binary_end {
                return Err("K14.C20 truncated IP prefix");
            }

            let hw_id = le16(b, cursor).ok_or("K14.C20 truncated hw_id")?;
            let instance = *b.get(cursor + 2).ok_or("K14.C20 truncated instance")?;
            let num_bases = *b.get(cursor + 3).ok_or("K14.C20 truncated base count")?;
            let major = *b.get(cursor + 4).ok_or("K14.C20 truncated IP major")?;
            let minor = *b.get(cursor + 5).ok_or("K14.C20 truncated IP minor")?;
            let revision = *b.get(cursor + 6).ok_or("K14.C20 truncated IP revision")?;
            let meta = *b.get(cursor + 7).ok_or("K14.C20 truncated IP metadata")?;
            if num_bases > native_gpu_c17::AMD_DISCOVERY_MAX_BASES {
                return Err("K14.C20 IP base-address count exceeds bounded policy");
            }
            let base_width = if top.base_addr_64_bit { 8usize } else { 4usize };
            let record_bytes = RADEON_C20_FIXED_IP_BYTES
                .checked_add((num_bases as usize).checked_mul(base_width).ok_or("K14.C20 base-list length overflow")?)
                .ok_or("K14.C20 IP record length overflow")?;
            if cursor.checked_add(record_bytes).ok_or("K14.C20 IP record end overflow")? > binary_end {
                return Err("K14.C20 IP base list exceeds discovery binary");
            }

            let base0_dwords = if num_bases == 0 {
                0
            } else if top.base_addr_64_bit {
                // Mirrors amdgpu: lower_32_bits(base64) & 0x3fffffff.
                (le64(b, cursor + RADEON_C20_FIXED_IP_BYTES)
                    .ok_or("K14.C20 truncated 64-bit base")? as u32 & 0x3fff_ffff) as u64
            } else {
                le32(b, cursor + RADEON_C20_FIXED_IP_BYTES)
                    .ok_or("K14.C20 truncated 32-bit base")? as u64
            };

            let (sub_revision, variant) = if top.ip_version >= 3 {
                (meta & 0x0f, meta >> 4)
            } else {
                (0, 0)
            };
            let candidate = ResolvedIp {
                found: true,
                hw_id,
                instance,
                major,
                minor,
                revision,
                sub_revision,
                variant,
                num_base_addresses: num_bases,
                base0_dwords,
            };

            match hw_id {
                AMD_GC_HWID => set_unique(&mut out.gc, candidate)?,
                AMD_SDMA0_HWID => set_unique(&mut out.sdma0, candidate)?,
                AMD_SDMA1_HWID => set_unique(&mut out.sdma1, candidate)?,
                AMD_SDMA2_HWID => set_unique(&mut out.sdma2, candidate)?,
                AMD_SDMA3_HWID => set_unique(&mut out.sdma3, candidate)?,
                _ => {}
            }
            cursor = cursor.checked_add(record_bytes).ok_or("K14.C20 IP cursor overflow")?;
        }
    }

    if !out.gc.found || out.gc.base0_dwords == 0 {
        return Err("K14.C20 verified snapshot has no usable instance-0 GC base");
    }
    if !out.sdma0.found || out.sdma0.base0_dwords == 0 {
        return Err("K14.C20 verified snapshot has no usable instance-0 SDMA0 base");
    }
    Ok(out)
}

fn self_test() -> Result<(), &'static str> {
    if K14C20_ABI_VERSION != 1
        || AMD_GC_HWID != 11
        || AMD_SDMA0_HWID != 42
        || AMD_SDMA1_HWID != 43
        || AMD_SDMA2_HWID != 68
        || AMD_SDMA3_HWID != 69
        || RADEON_C20_MMIO_WRITES_ALLOWED
        || RADEON_C20_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C20_COMMAND_SUBMIT_ALLOWED
        || RADEON_C20_BUS_MASTER_ALLOWED
    {
        return Err("K14.C20 source/fail-closed constants invalid");
    }

    // Synthetic v4 / 64-bit-base discovery image with one GC and one SDMA0.
    let mut b = [0u8; 256];
    b[0..4].copy_from_slice(&native_gpu_c17::AMD_DISCOVERY_BINARY_SIGNATURE.to_le_bytes());
    b[4..6].copy_from_slice(&1u16.to_le_bytes());
    b[10..12].copy_from_slice(&256u16.to_le_bytes());
    b[12..14].copy_from_slice(&64u16.to_le_bytes());
    b[16..18].copy_from_slice(&192u16.to_le_bytes());
    b[64..68].copy_from_slice(&native_gpu_c17::AMD_DISCOVERY_TABLE_SIGNATURE.to_le_bytes());
    b[68..70].copy_from_slice(&4u16.to_le_bytes());
    b[70..72].copy_from_slice(&192u16.to_le_bytes());
    b[76..78].copy_from_slice(&1u16.to_le_bytes());
    // die_info[0] => die 0, absolute binary offset 160.
    b[78..80].copy_from_slice(&0u16.to_le_bytes());
    b[80..82].copy_from_slice(&160u16.to_le_bytes());
    b[142] = 1; // v4: 64-bit base-address encoding.
    b[160..162].copy_from_slice(&0u16.to_le_bytes());
    b[162..164].copy_from_slice(&2u16.to_le_bytes());

    let gc = 164usize;
    b[gc..gc + 2].copy_from_slice(&AMD_GC_HWID.to_le_bytes());
    b[gc + 2] = 0; b[gc + 3] = 1; b[gc + 4] = 12; b[gc + 5] = 0; b[gc + 6] = 1;
    b[gc + 7] = 0x21; // variant 2, subrevision 1.
    b[gc + 8..gc + 16].copy_from_slice(&0xabcd_0000_0000_5000u64.to_le_bytes());

    let sdma = gc + 16;
    b[sdma..sdma + 2].copy_from_slice(&AMD_SDMA0_HWID.to_le_bytes());
    b[sdma + 2] = 0; b[sdma + 3] = 1; b[sdma + 4] = 7; b[sdma + 5] = 0; b[sdma + 6] = 0;
    b[sdma + 8..sdma + 16].copy_from_slice(&0x1234_0000_0000_6000u64.to_le_bytes());

    let r = resolve_verified_snapshot(&b)?;
    if !r.valid || r.ip_version != 4 || !r.base_addr_64_bit || r.records_scanned != 2
        || r.gc.base0_dwords != 0x5000 || r.sdma0.base0_dwords != 0x6000
        || r.gc.major != 12 || r.gc.sub_revision != 1 || r.gc.variant != 2
    {
        return Err("K14.C20 synthetic IP-record resolver self-test failed");
    }
    Ok(())
}

pub fn initialize() -> Result<C20State, &'static str> {
    self_test()?;
    let c9 = native_gpu_c9::state();
    let c19 = native_gpu_c19::state();
    let mut s = C20State {
        amd_present: c9.amd_present,
        navi48: c9.profile == native_gpu_c9::ProfileId::Navi48Rx9070,
        c19_snapshot_verified: c19.live_snapshot_verified,
        parser_ready: true,
        source_hwids_imported: true,
        snapshot_fingerprint: c19.snapshot_fingerprint,
        device_id: c9.device_id,
        revision: c9.revision,
        ..C20State::EMPTY
    };

    serial::println(format_args!(
        "[C20IP] AMD live IP-record resolver: GC_HWID={} SDMA_HWIDS={}/{}/{}/{} max_dies={} max_ip_per_die={} max_bases={} base64_low32_mask={:#010x} source_hwids=true",
        AMD_GC_HWID, AMD_SDMA0_HWID, AMD_SDMA1_HWID, AMD_SDMA2_HWID, AMD_SDMA3_HWID,
        native_gpu_c17::AMD_DISCOVERY_MAX_DIES, RADEON_C20_MAX_IPS_PER_DIE,
        native_gpu_c17::AMD_DISCOVERY_MAX_BASES, 0x3fff_ffffu32
    ));
    serial::println(format_args!(
        "[C20PG] exact-base policy: require=C19_checksum_verified_snapshot+bounded_die_walk+instance0_GC+instance0_SDMA0+nonzero_base; guessed_bases=false MMIO_write=false firmware=false submit=false bus_master_enable=false"
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C20HW] AMD exact IP bases: present=false qemu_deferred=true snapshot=false records=0 GC=false SDMA0=false exact_base_set=false fallback=true"
        ));
    } else if !s.c19_snapshot_verified {
        serial::println(format_args!(
            "[C20HW] AMD exact IP bases: present=true devid={:#06x} snapshot=false records=0 GC=false SDMA0=false exact_base_set=false reason=C19_snapshot_not_verified fallback=true",
            s.device_id
        ));
    } else {
        let resolved = native_gpu_c19::with_verified_snapshot(resolve_verified_snapshot)
            .ok_or("K14.C20 C19 verified snapshot unavailable")??;
        s.records_scanned = resolved.records_scanned;
        s.ip_version = resolved.ip_version;
        s.num_dies = resolved.num_dies;
        s.base_addr_64_bit = resolved.base_addr_64_bit;
        s.gc_resolved = resolved.gc.found;
        s.sdma0_resolved = resolved.sdma0.found;
        s.sdma1_resolved = resolved.sdma1.found;
        s.sdma2_resolved = resolved.sdma2.found;
        s.sdma3_resolved = resolved.sdma3.found;
        s.gc_base_dwords = resolved.gc.base0_dwords;
        s.sdma0_base_dwords = resolved.sdma0.base0_dwords;
        s.sdma1_base_dwords = resolved.sdma1.base0_dwords;
        s.sdma2_base_dwords = resolved.sdma2.base0_dwords;
        s.sdma3_base_dwords = resolved.sdma3.base0_dwords;
        s.gc_major = resolved.gc.major;
        s.gc_minor = resolved.gc.minor;
        s.gc_revision = resolved.gc.revision;
        s.exact_base_set_ready = s.gc_resolved && s.sdma0_resolved
            && s.gc_base_dwords != 0 && s.sdma0_base_dwords != 0;
        // For the Navi48 profile, the live GC record must identify GFX12 before
        // the C16 reviewed GFX12 target may consume this base in a later stage.
        s.c16_promotion_input_ready = s.navi48 && s.exact_base_set_ready && s.gc_major == 12;
        serial::println(format_args!(
            "[C20HW] AMD exact IP bases: present=true navi48={} devid={:#06x} snapshot=true fingerprint={:#018x} ip_v={} dies={} records={} base64={} GC=v{}.{}.{}@{:#x} SDMA0={:#x} SDMA1={:#x} SDMA2={:#x} SDMA3={:#x} exact_base_set={} c16_input={} fallback=true",
            s.navi48, s.device_id, s.snapshot_fingerprint, s.ip_version, s.num_dies,
            s.records_scanned, s.base_addr_64_bit, s.gc_major, s.gc_minor, s.gc_revision,
            s.gc_base_dwords, s.sdma0_base_dwords, s.sdma1_base_dwords,
            s.sdma2_base_dwords, s.sdma3_base_dwords, s.exact_base_set_ready,
            s.c16_promotion_input_ready
        ));
    }

    if s.exact_base_set_ready && !s.c19_snapshot_verified {
        return Err("K14.C20 exact base set resolved without C19 verified snapshot");
    }
    if s.c16_promotion_input_ready && (!s.navi48 || s.gc_major != 12 || !s.exact_base_set_ready) {
        return Err("K14.C20 C16 promotion input qualified without Navi48 GFX12 base proof");
    }
    if s.mmio_write_enabled || s.firmware_upload_enabled || s.command_submit_enabled || s.radeon_bus_master_enabled
        || RADEON_C20_MMIO_WRITES_ALLOWED || RADEON_C20_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C20_COMMAND_SUBMIT_ALLOWED || RADEON_C20_BUS_MASTER_ALLOWED
    {
        return Err("K14.C20 destructive capability promoted early");
    }

    serial::println(format_args!(
        "[C20RD] K14.C20 exact-base gate ready: amd_present={} navi48={} C19_verified={} parser={} hwids={} ip_v={} dies={} records={} base64={} GC={} GC_base={:#x} SDMA0={} SDMA0_base={:#x} exact_base_set={} c16_input={} fingerprint={:#018x} writes=false upload=false submit=false bus_master=false fallback=true",
        s.amd_present, s.navi48, s.c19_snapshot_verified, s.parser_ready,
        s.source_hwids_imported, s.ip_version, s.num_dies, s.records_scanned,
        s.base_addr_64_bit, s.gc_resolved, s.gc_base_dwords, s.sdma0_resolved,
        s.sdma0_base_dwords, s.exact_base_set_ready, s.c16_promotion_input_ready,
        s.snapshot_fingerprint
    ));
    *STATE.lock() = s;
    Ok(s)
}

pub fn state() -> C20State { *STATE.lock() }

pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.device_id) << 40)
        | (u64::from(s.revision) << 32)
        | (u64::from(s.gc_major) << 24)
        | (u64::from(s.records_scanned.min(255) as u8) << 16);
    for (bit, on) in [
        s.amd_present,
        s.navi48,
        s.c19_snapshot_verified,
        s.parser_ready,
        s.gc_resolved,
        s.sdma0_resolved,
        s.exact_base_set_ready,
        s.base_addr_64_bit,
        s.fallback_armed,
        s.c16_promotion_input_ready,
    ].into_iter().enumerate() {
        if on { v |= 1u64 << bit; }
    }
    v
}
