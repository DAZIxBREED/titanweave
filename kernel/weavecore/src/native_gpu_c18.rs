//! K14.C18 AMD IP Discovery snapshot checksum and bounded-acquisition gate.
//!
//! C18 imports the checksum semantics and discovery-TMR location contract used
//! by upstream AMDGPU. It adds a bounded verifier for the binary checksum and
//! IP-discovery table checksum, including a synthetic self-test. The physical
//! TMR/VRAM fetch is deliberately not promoted yet: C18 proves the acquisition
//! contract and verifier while keeping live Radeon memory access fail-closed.

use crate::{native_gpu_c9, native_gpu_c17, serial, sync::SpinLock};

pub const K14C18_ABI_VERSION: u32 = 1;
pub const AMD_DISCOVERY_BINARY_CHECKSUM_OFFSET: usize = 8;
pub const AMD_DISCOVERY_CHECKSUM_PAYLOAD_OFFSET: usize = 10;
pub const AMD_DISCOVERY_TMR_SIZE: u32 = 10 * 1024;
pub const AMD_DISCOVERY_TMR_OFFSET: u64 = 64 * 1024;
pub const AMD_DISCOVERY_MM_IP_VERSION: u32 = 0x16A00;
pub const AMD_DISCOVERY_MM_RCC_CONFIG_MEMSIZE: u32 = 0x0DE3;
pub const AMD_DISCOVERY_MM_DRIVER_SCRATCH_0: u32 = 0x0094;
pub const AMD_DISCOVERY_MM_DRIVER_SCRATCH_1: u32 = 0x0095;
pub const AMD_DISCOVERY_MM_DRIVER_SCRATCH_2: u32 = 0x0096;

pub const RADEON_C18_LIVE_TMR_READ_ALLOWED: bool = false;
pub const RADEON_C18_LIVE_VRAM_READ_ALLOWED: bool = false;
pub const RADEON_C18_MMIO_WRITE_ALLOWED: bool = false;
pub const RADEON_C18_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C18_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C18_BUS_MASTER_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct SnapshotVerification {
    pub valid: bool,
    pub binary_checksum_expected: u16,
    pub binary_checksum_calculated: u16,
    pub ip_checksum_expected: u16,
    pub ip_checksum_calculated: u16,
    pub binary_size: u16,
    pub ip_offset: u16,
    pub ip_size: u16,
    pub fingerprint: u64,
}
impl SnapshotVerification {
    pub const EMPTY: Self = Self { valid:false,binary_checksum_expected:0,binary_checksum_calculated:0,ip_checksum_expected:0,ip_checksum_calculated:0,binary_size:0,ip_offset:0,ip_size:0,fingerprint:0 };
}

#[derive(Clone, Copy, Debug)]
pub struct C18State {
    pub amd_present: bool,
    pub navi48: bool,
    pub c17_ready: bool,
    pub checksum_engine_ready: bool,
    pub tmr_contract_imported: bool,
    pub synthetic_checksum_selftest_passed: bool,
    pub acquisition_promoted: bool,
    pub live_snapshot_acquired: bool,
    pub live_binary_checksum_verified: bool,
    pub live_ip_checksum_verified: bool,
    pub live_snapshot_verified: bool,
    pub exact_gc_base_resolved: bool,
    pub mmio_write_enabled: bool,
    pub firmware_upload_enabled: bool,
    pub command_submit_enabled: bool,
    pub radeon_bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub device_id: u16,
    pub revision: u8,
}
impl C18State {
    pub const EMPTY: Self = Self { amd_present:false,navi48:false,c17_ready:false,checksum_engine_ready:false,tmr_contract_imported:false,synthetic_checksum_selftest_passed:false,acquisition_promoted:false,live_snapshot_acquired:false,live_binary_checksum_verified:false,live_ip_checksum_verified:false,live_snapshot_verified:false,exact_gc_base_resolved:false,mmio_write_enabled:false,firmware_upload_enabled:false,command_submit_enabled:false,radeon_bus_master_enabled:false,fallback_armed:true,device_id:0,revision:0 };
}
static STATE: SpinLock<C18State> = SpinLock::new(C18State::EMPTY);

fn le16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(o)?, *b.get(o + 1)?]))
}

pub fn byte_sum16(data: &[u8]) -> u16 {
    data.iter().fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte)))
}

fn mix_fingerprint(mut h: u64, v: u64) -> u64 {
    h ^= v.wrapping_add(0x9e37_79b9_7f4a_7c15).wrapping_add(h << 6).wrapping_add(h >> 2);
    h
}

/// Verify the AMD discovery binary checksum and the IP_DISCOVERY table checksum.
/// The checksum algorithm is a wrapping 16-bit sum of bytes, matching AMDGPU.
pub fn verify_snapshot_checksums(b: &[u8]) -> Result<SnapshotVerification, &'static str> {
    let parsed = native_gpu_c17::parse_discovery_snapshot(b)?;
    let binary_size = parsed.binary_size as usize;
    if binary_size < AMD_DISCOVERY_CHECKSUM_PAYLOAD_OFFSET || binary_size > b.len() {
        return Err("K14.C18 binary checksum range invalid");
    }
    let expected_binary = le16(b, AMD_DISCOVERY_BINARY_CHECKSUM_OFFSET).ok_or("K14.C18 binary checksum missing")?;
    let calculated_binary = byte_sum16(&b[AMD_DISCOVERY_CHECKSUM_PAYLOAD_OFFSET..binary_size]);
    if calculated_binary != expected_binary {
        return Err("K14.C18 AMD discovery binary checksum mismatch");
    }

    let table_base = if parsed.version_major >= 2 { 16usize } else { 12usize };
    let expected_ip = le16(b, table_base + 2).ok_or("K14.C18 IP checksum missing")?;
    let ip_off = parsed.ip_table_offset as usize;
    let header_size = le16(b, ip_off + 6).ok_or("K14.C18 IP table header size missing")? as usize;
    if header_size == 0 || header_size > parsed.ip_table_size as usize || ip_off.checked_add(header_size).ok_or("K14.C18 IP checksum overflow")? > binary_size {
        return Err("K14.C18 IP checksum range invalid");
    }
    let calculated_ip = byte_sum16(&b[ip_off..ip_off + header_size]);
    if calculated_ip != expected_ip {
        return Err("K14.C18 AMD discovery IP table checksum mismatch");
    }

    let mut fp = 0xC18D_15C0_0000_0001u64;
    fp = mix_fingerprint(fp, u64::from(parsed.binary_size));
    fp = mix_fingerprint(fp, u64::from(parsed.ip_table_offset));
    fp = mix_fingerprint(fp, u64::from(header_size as u32));
    fp = mix_fingerprint(fp, (u64::from(expected_binary) << 16) | u64::from(expected_ip));
    Ok(SnapshotVerification {
        valid: true,
        binary_checksum_expected: expected_binary,
        binary_checksum_calculated: calculated_binary,
        ip_checksum_expected: expected_ip,
        ip_checksum_calculated: calculated_ip,
        binary_size: parsed.binary_size,
        ip_offset: parsed.ip_table_offset,
        ip_size: header_size as u16,
        fingerprint: fp,
    })
}

fn checksum_self_test() -> Result<SnapshotVerification, &'static str> {
    let mut b = [0u8; 128];
    b[0..4].copy_from_slice(&native_gpu_c17::AMD_DISCOVERY_BINARY_SIGNATURE.to_le_bytes());
    b[4..6].copy_from_slice(&1u16.to_le_bytes());
    b[10..12].copy_from_slice(&128u16.to_le_bytes());
    // v1 table_list[IP_DISCOVERY]: offset=64, checksum filled below, size=64.
    b[12..14].copy_from_slice(&64u16.to_le_bytes());
    b[16..18].copy_from_slice(&64u16.to_le_bytes());
    b[64..68].copy_from_slice(&native_gpu_c17::AMD_DISCOVERY_TABLE_SIGNATURE.to_le_bytes());
    b[68..70].copy_from_slice(&3u16.to_le_bytes());
    b[70..72].copy_from_slice(&64u16.to_le_bytes());
    b[76..78].copy_from_slice(&1u16.to_le_bytes());
    let ip_sum = byte_sum16(&b[64..128]);
    b[14..16].copy_from_slice(&ip_sum.to_le_bytes());
    let binary_sum = byte_sum16(&b[AMD_DISCOVERY_CHECKSUM_PAYLOAD_OFFSET..128]);
    b[8..10].copy_from_slice(&binary_sum.to_le_bytes());
    let proof = verify_snapshot_checksums(&b)?;
    if !proof.valid || proof.binary_size != 128 || proof.ip_offset != 64 || proof.ip_size != 64 || proof.fingerprint == 0 {
        return Err("K14.C18 checksum self-test mismatch");
    }
    Ok(proof)
}

pub fn initialize() -> Result<C18State, &'static str> {
    if K14C18_ABI_VERSION != 1 || RADEON_C18_LIVE_TMR_READ_ALLOWED || RADEON_C18_LIVE_VRAM_READ_ALLOWED || RADEON_C18_MMIO_WRITE_ALLOWED || RADEON_C18_FIRMWARE_UPLOAD_ALLOWED || RADEON_C18_COMMAND_SUBMIT_ALLOWED || RADEON_C18_BUS_MASTER_ALLOWED {
        return Err("K14.C18 fail-closed constants invalid");
    }
    let proof = checksum_self_test()?;
    let c9 = native_gpu_c9::state();
    let c17 = native_gpu_c17::state();
    let s = C18State {
        amd_present: c9.amd_present,
        navi48: c9.profile == native_gpu_c9::ProfileId::Navi48Rx9070,
        c17_ready: c17.parser_ready && c17.synthetic_selftest_passed,
        checksum_engine_ready: true,
        tmr_contract_imported: true,
        synthetic_checksum_selftest_passed: true,
        device_id: c9.device_id,
        revision: c9.revision,
        ..C18State::EMPTY
    };

    serial::println(format_args!("[C18CK] discovery checksum verifier: algorithm=wrapping_u16_byte_sum binary_payload_offset={} synthetic_binary={:#06x} synthetic_ip={:#06x} fingerprint={:#018x} selftest=true", AMD_DISCOVERY_CHECKSUM_PAYLOAD_OFFSET, proof.binary_checksum_calculated, proof.ip_checksum_calculated, proof.fingerprint));
    serial::println(format_args!("[C18PG] snapshot acquisition policy: source=AMD_discovery_TMR default_size={} default_tail_offset={} scratch_regs={:#x}/{:#x}/{:#x} live_TMR_read=false live_VRAM_read=false checksum_required=binary+IP table guessed_offsets=false MMIO_write=false firmware=false submit=false bus_master_enable=false", AMD_DISCOVERY_TMR_SIZE, AMD_DISCOVERY_TMR_OFFSET, AMD_DISCOVERY_MM_DRIVER_SCRATCH_0, AMD_DISCOVERY_MM_DRIVER_SCRATCH_1, AMD_DISCOVERY_MM_DRIVER_SCRATCH_2));
    if !s.amd_present {
        serial::println(format_args!("[C18HW] AMD discovery snapshot: present=false qemu_deferred=true checksum_engine=true acquisition=false snapshot=false verified=false fallback=true"));
    } else if s.navi48 {
        serial::println(format_args!("[C18HW] AMD discovery snapshot: present=true navi48=true devid={:#06x} checksum_engine=true acquisition=false snapshot=false verified=false reason=physical_TMR_fetch_not_promoted fallback=true", s.device_id));
    } else {
        serial::println(format_args!("[C18HW] AMD discovery snapshot: present=true navi48=false devid={:#06x} checksum_engine=true acquisition=false snapshot=false verified=false fallback=true", s.device_id));
    }

    if s.live_binary_checksum_verified && !s.live_snapshot_acquired { return Err("K14.C18 binary checksum verified without snapshot"); }
    if s.live_ip_checksum_verified && !s.live_snapshot_acquired { return Err("K14.C18 IP checksum verified without snapshot"); }
    if s.live_snapshot_verified && !(s.live_snapshot_acquired && s.live_binary_checksum_verified && s.live_ip_checksum_verified) { return Err("K14.C18 snapshot verified without both checksums"); }
    if s.exact_gc_base_resolved && !s.live_snapshot_verified { return Err("K14.C18 exact GC base resolved without verified snapshot"); }
    if s.mmio_write_enabled || s.firmware_upload_enabled || s.command_submit_enabled || s.radeon_bus_master_enabled { return Err("K14.C18 destructive capability promoted early"); }

    serial::println(format_args!("[C18RD] K14.C18 snapshot-verification gate ready: amd_present={} navi48={} C17_ready={} checksum_engine={} TMR_contract={} selftest={} acquisition={} snapshot={} binary_ck={} ip_ck={} verified={} gc_base={} fallback=true", s.amd_present,s.navi48,s.c17_ready,s.checksum_engine_ready,s.tmr_contract_imported,s.synthetic_checksum_selftest_passed,s.acquisition_promoted,s.live_snapshot_acquired,s.live_binary_checksum_verified,s.live_ip_checksum_verified,s.live_snapshot_verified,s.exact_gc_base_resolved));
    *STATE.lock() = s;
    Ok(s)
}

pub fn state() -> C18State { *STATE.lock() }
pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.device_id) << 32) | (u64::from(s.revision) << 24);
    for (bit, on) in [s.amd_present,s.navi48,s.c17_ready,s.checksum_engine_ready,s.tmr_contract_imported,s.synthetic_checksum_selftest_passed,s.live_snapshot_acquired,s.live_binary_checksum_verified,s.live_ip_checksum_verified,s.live_snapshot_verified,s.exact_gc_base_resolved,s.fallback_armed].into_iter().enumerate() {
        if on { v |= 1u64 << bit; }
    }
    v
}
