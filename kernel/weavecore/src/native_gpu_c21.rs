//! K14.C21 reviewed GFX12 target rebind and identity-write gate.
//!
//! C19/C20 close the discovery gap that kept the frozen C14-C16 Navi48 path
//! fail-closed. C21 does not rewrite those frozen milestones. Instead it derives
//! a new post-discovery gate from the same safety invariants plus the later,
//! stronger proofs: verified Navi48 identity, live translated DMA domain,
//! C16's source-reviewed GFX12 SCRATCH_REG0 semantics, a checksum-qualified C19
//! discovery snapshot, and C20's exact live GFX12 GC/SDMA resolution.
//!
//! The generated GFX12 register definition is exact:
//!   regSCRATCH_REG0          = 0x2040 dwords
//!   regSCRATCH_REG0_BASE_IDX = 1
//! AMDGPU's SOC15 register rule is GC.base[BASE_IDX] + register. C21 therefore
//! re-opens only the already verified C19 snapshot to resolve GC base slot 1,
//! cross-checks slot 0 and the GC version against frozen C20, then forms the
//! reviewed BAR5 byte address. No static/guessed Navi48 base is used.
//!
//! If every physical gate passes, C21 permits exactly one GPU-visible identity
//! transaction: read current SCRATCH_REG0 -> write the same u32 -> bounded exact
//! readback. One restore write is allowed only after failed readback, and that
//! run is still considered unqualified. Arbitrary MMIO writes, MM_INDEX fallback,
//! BAR resizing, firmware upload, command submission, and Radeon bus-master
//! enable remain forbidden.

use crate::{
    memory::FrameAllocator,
    native_gpu_binding,
    native_gpu_c6,
    native_gpu_c9,
    native_gpu_c16,
    native_gpu_c17,
    native_gpu_c19,
    native_gpu_c20,
    paging,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C21_ABI_VERSION: u32 = 1;

// Generated GFX12 definitions from gc_12_0_0_offset.h.
pub const GFX12_SCRATCH_REG0_DWORD: u32 = 0x2040;
pub const GFX12_SCRATCH_REG0_BASE_IDX: u8 = 1;

pub const RADEON_C21_MMIO_BAR_INDEX: u8 = 5;
pub const RADEON_C21_PAGE_BYTES: u64 = 4096;
pub const RADEON_C21_MAX_READBACK_POLLS: u8 = 32;
pub const RADEON_C21_MAX_MMIO_WRITES: u8 = 2; // identity + one bounded restore
pub const RADEON_C21_IDENTITY_MMIO_WRITE_ALLOWED: bool = true;
pub const RADEON_C21_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C21_MM_INDEX_FALLBACK_ALLOWED: bool = false;
pub const RADEON_C21_BAR_RESIZE_ALLOWED: bool = false;
pub const RADEON_C21_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C21_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C21_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C21_GUESSED_REGISTER_ALLOWED: bool = false;
pub const RADEON_C21_GUESSED_BASE_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewedTarget {
    pub valid: bool,
    pub gc_major: u8,
    pub gc_minor: u8,
    pub gc_revision: u8,
    pub gc_segment0_base_dwords: u64,
    pub gc_segment1_base_dwords: u64,
    pub target_dword_offset: u64,
    pub target_byte_offset: u64,
}
impl ReviewedTarget {
    pub const EMPTY: Self = Self {
        valid: false,
        gc_major: 0,
        gc_minor: 0,
        gc_revision: 0,
        gc_segment0_base_dwords: 0,
        gc_segment1_base_dwords: 0,
        target_dword_offset: 0,
        target_byte_offset: 0,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct C21State {
    pub amd_present: bool,
    pub navi48: bool,
    pub profile_verified: bool,
    pub exact_domain_live: bool,
    pub c16_target_reviewed: bool,
    pub c19_snapshot_verified: bool,
    pub c20_exact_bases_ready: bool,
    pub source_register_imported: bool,
    pub source_base_index_imported: bool,
    pub gc_segment1_resolved: bool,
    pub gc_segment0_crosschecked: bool,
    pub gc_segment1_base_dwords: u64,
    pub target_dword_offset: u64,
    pub target_byte_offset: u64,
    pub bar5_ready: bool,
    pub memory_decode_before_on: bool,
    pub memory_decode_after_on: bool,
    pub bus_master_before_off: bool,
    pub bus_master_after_off: bool,
    pub transaction_eligible: bool,
    pub transaction_attempted: bool,
    pub transaction_verified: bool,
    pub rollback_attempted: bool,
    pub rollback_verified: bool,
    pub writes_performed: u8,
    pub readback_polls: u8,
    pub value_before: u32,
    pub value_after: u32,
    pub snapshot_fingerprint: u64,
    pub transaction_fingerprint: u64,
    pub arbitrary_mmio_write_enabled: bool,
    pub mm_index_fallback_used: bool,
    pub bar_resize_used: bool,
    pub firmware_upload_enabled: bool,
    pub command_submit_enabled: bool,
    pub radeon_bus_master_enabled: bool,
    pub fallback_armed: bool,
    pub device_id: u16,
    pub revision: u8,
}
impl C21State {
    pub const EMPTY: Self = Self {
        amd_present: false,
        navi48: false,
        profile_verified: false,
        exact_domain_live: false,
        c16_target_reviewed: false,
        c19_snapshot_verified: false,
        c20_exact_bases_ready: false,
        source_register_imported: false,
        source_base_index_imported: false,
        gc_segment1_resolved: false,
        gc_segment0_crosschecked: false,
        gc_segment1_base_dwords: 0,
        target_dword_offset: 0,
        target_byte_offset: 0,
        bar5_ready: false,
        memory_decode_before_on: false,
        memory_decode_after_on: false,
        bus_master_before_off: false,
        bus_master_after_off: false,
        transaction_eligible: false,
        transaction_attempted: false,
        transaction_verified: false,
        rollback_attempted: false,
        rollback_verified: false,
        writes_performed: 0,
        readback_polls: 0,
        value_before: 0,
        value_after: 0,
        snapshot_fingerprint: 0,
        transaction_fingerprint: 0,
        arbitrary_mmio_write_enabled: false,
        mm_index_fallback_used: false,
        bar_resize_used: false,
        firmware_upload_enabled: false,
        command_submit_enabled: false,
        radeon_bus_master_enabled: false,
        fallback_armed: true,
        device_id: 0,
        revision: 0,
    };
}

static STATE: SpinLock<C21State> = SpinLock::new(C21State::EMPTY);

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
fn decode_reg_base(b: &[u8], o: usize, base64: bool) -> Result<u64, &'static str> {
    if base64 {
        Ok((le64(b, o).ok_or("K14.C21 truncated 64-bit GC base")? as u32 & 0x3fff_ffff) as u64)
    } else {
        Ok(le32(b, o).ok_or("K14.C21 truncated 32-bit GC base")? as u64)
    }
}

/// Rebind the reviewed GFX12 SCRATCH_REG0 target from a checksum-qualified AMD
/// discovery snapshot. Frozen C20 is deliberately reused as the first parser
/// and exact-GC/SDMA proof, then C21 extracts the generated BASE_IDX=1 slot.
pub fn resolve_gfx12_scratch_reg0(b: &[u8]) -> Result<ReviewedTarget, &'static str> {
    let c20 = native_gpu_c20::resolve_verified_snapshot(b)?;
    if !c20.valid || !c20.gc.found || c20.gc.major != 12 || c20.gc.base0_dwords == 0 {
        return Err("K14.C21 C20 snapshot proof is not a usable GFX12 GC record");
    }
    let top = native_gpu_c17::parse_discovery_snapshot(b)?;
    if !top.valid || top.binary_size as usize > b.len() {
        return Err("K14.C21 invalid verified discovery envelope");
    }
    let ip_off = top.ip_table_offset as usize;
    let binary_end = top.binary_size as usize;
    let mut out = ReviewedTarget::EMPTY;

    for die_index in 0..top.num_dies {
        let info = ip_off.checked_add(14)
            .and_then(|v| v.checked_add(die_index as usize * 4))
            .ok_or("K14.C21 die-info offset overflow")?;
        let listed_die = le16(b, info).ok_or("K14.C21 truncated die-info id")?;
        let die_offset = le16(b, info + 2).ok_or("K14.C21 truncated die-info offset")? as usize;
        if listed_die != die_index {
            return Err("K14.C21 die-info id mismatch");
        }
        if die_offset < ip_off || die_offset.checked_add(4).ok_or("K14.C21 die-header overflow")? > binary_end {
            return Err("K14.C21 die header outside discovery binary");
        }
        let die_id = le16(b, die_offset).ok_or("K14.C21 truncated die header")?;
        let num_ips = le16(b, die_offset + 2).ok_or("K14.C21 truncated IP count")?;
        if die_id != die_index || num_ips > native_gpu_c20::RADEON_C20_MAX_IPS_PER_DIE {
            return Err("K14.C21 invalid bounded die/IP count");
        }
        let mut cursor = die_offset + 4;
        for _ in 0..num_ips {
            if cursor.checked_add(native_gpu_c20::RADEON_C20_FIXED_IP_BYTES)
                .ok_or("K14.C21 IP-prefix overflow")? > binary_end {
                return Err("K14.C21 truncated IP prefix");
            }
            let hw_id = le16(b, cursor).ok_or("K14.C21 truncated hw_id")?;
            let instance = *b.get(cursor + 2).ok_or("K14.C21 truncated instance")?;
            let num_bases = *b.get(cursor + 3).ok_or("K14.C21 truncated base count")?;
            let major = *b.get(cursor + 4).ok_or("K14.C21 truncated GC major")?;
            let minor = *b.get(cursor + 5).ok_or("K14.C21 truncated GC minor")?;
            let revision = *b.get(cursor + 6).ok_or("K14.C21 truncated GC revision")?;
            if num_bases > native_gpu_c17::AMD_DISCOVERY_MAX_BASES {
                return Err("K14.C21 IP base count exceeds bounded policy");
            }
            let width = if top.base_addr_64_bit { 8usize } else { 4usize };
            let record_bytes = native_gpu_c20::RADEON_C20_FIXED_IP_BYTES
                .checked_add((num_bases as usize).checked_mul(width).ok_or("K14.C21 base-span overflow")?)
                .ok_or("K14.C21 IP-record length overflow")?;
            if cursor.checked_add(record_bytes).ok_or("K14.C21 IP-record end overflow")? > binary_end {
                return Err("K14.C21 truncated IP base array");
            }

            if hw_id == native_gpu_c20::AMD_GC_HWID && instance == 0 {
                if num_bases <= GFX12_SCRATCH_REG0_BASE_IDX {
                    return Err("K14.C21 GC record lacks generated BASE_IDX=1 slot");
                }
                let base_start = cursor + native_gpu_c20::RADEON_C20_FIXED_IP_BYTES;
                let base0 = decode_reg_base(b, base_start, top.base_addr_64_bit)?;
                let base1 = decode_reg_base(
                    b,
                    base_start + usize::from(GFX12_SCRATCH_REG0_BASE_IDX) * width,
                    top.base_addr_64_bit,
                )?;
                if base0 != c20.gc.base0_dwords || major != c20.gc.major
                    || minor != c20.gc.minor || revision != c20.gc.revision {
                    return Err("K14.C21 GC slot/version cross-check disagrees with C20 resolver");
                }
                let target_dword_offset = base1.checked_add(u64::from(GFX12_SCRATCH_REG0_DWORD))
                    .ok_or("K14.C21 target dword offset overflow")?;
                let target_byte_offset = target_dword_offset.checked_mul(4)
                    .ok_or("K14.C21 target byte offset overflow")?;
                if base1 == 0 || target_byte_offset > 0x00ff_ffff || target_byte_offset & 3 != 0 {
                    return Err("K14.C21 reviewed target outside bounded direct-MMIO aperture");
                }
                let candidate = ReviewedTarget {
                    valid: true,
                    gc_major: major,
                    gc_minor: minor,
                    gc_revision: revision,
                    gc_segment0_base_dwords: base0,
                    gc_segment1_base_dwords: base1,
                    target_dword_offset,
                    target_byte_offset,
                };
                if !out.valid {
                    out = candidate;
                } else if out != candidate {
                    return Err("K14.C21 conflicting duplicate GC instance-0 target");
                }
            }
            cursor = cursor.checked_add(record_bytes).ok_or("K14.C21 IP cursor overflow")?;
        }
    }
    if !out.valid || out.gc_segment1_base_dwords == 0 {
        return Err("K14.C21 verified snapshot lacks exact GC segment-1 base");
    }
    Ok(out)
}

fn selected_function() -> pci::PciFunction {
    let b = native_gpu_binding::state();
    pci::PciFunction {
        bus: b.selected_bus,
        device: b.selected_device,
        function: b.selected_function,
        vendor_id: 0,
        device_id: 0,
        class_code: 0,
        subclass: 0,
        programming_interface: 0,
        revision: 0,
        header_type: 0,
    }
}

fn transaction_fingerprint(device: u16, revision: u8, target: u64, before: u32, after: u32, writes: u8) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for byte in device.to_le_bytes().into_iter()
        .chain([revision])
        .chain(target.to_le_bytes())
        .chain(before.to_le_bytes())
        .chain(after.to_le_bytes())
        .chain([writes]) {
        h ^= u64::from(byte);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn self_test() -> Result<(), &'static str> {
    if K14C21_ABI_VERSION != 1
        || GFX12_SCRATCH_REG0_DWORD != 0x2040
        || GFX12_SCRATCH_REG0_BASE_IDX != 1
        || RADEON_C21_MMIO_BAR_INDEX != 5
        || RADEON_C21_MAX_READBACK_POLLS != 32
        || RADEON_C21_MAX_MMIO_WRITES != 2
        || !RADEON_C21_IDENTITY_MMIO_WRITE_ALLOWED
        || RADEON_C21_ARBITRARY_MMIO_WRITES_ALLOWED
        || RADEON_C21_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C21_BAR_RESIZE_ALLOWED
        || RADEON_C21_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C21_COMMAND_SUBMIT_ALLOWED
        || RADEON_C21_BUS_MASTER_ALLOWED
        || RADEON_C21_GUESSED_REGISTER_ALLOWED
        || RADEON_C21_GUESSED_BASE_ALLOWED {
        return Err("K14.C21 reviewed-target/fail-closed constants invalid");
    }

    // Synthetic v4 image with GC base0 and deliberately different base1.
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
    b[78..80].copy_from_slice(&0u16.to_le_bytes());
    b[80..82].copy_from_slice(&160u16.to_le_bytes());
    b[142] = 1;
    b[160..162].copy_from_slice(&0u16.to_le_bytes());
    b[162..164].copy_from_slice(&2u16.to_le_bytes());
    let gc = 164usize;
    b[gc..gc + 2].copy_from_slice(&native_gpu_c20::AMD_GC_HWID.to_le_bytes());
    b[gc + 2] = 0; b[gc + 3] = 2; b[gc + 4] = 12; b[gc + 5] = 0; b[gc + 6] = 1;
    b[gc + 8..gc + 16].copy_from_slice(&0xabcd_0000_0000_5000u64.to_le_bytes());
    b[gc + 16..gc + 24].copy_from_slice(&0x1234_0000_0000_7000u64.to_le_bytes());
    let sdma = gc + 24;
    b[sdma..sdma + 2].copy_from_slice(&native_gpu_c20::AMD_SDMA0_HWID.to_le_bytes());
    b[sdma + 2] = 0; b[sdma + 3] = 1; b[sdma + 4] = 7; b[sdma + 5] = 0; b[sdma + 6] = 0;
    b[sdma + 8..sdma + 16].copy_from_slice(&0x5678_0000_0000_6000u64.to_le_bytes());
    let target = resolve_gfx12_scratch_reg0(&b)?;
    if !target.valid || target.gc_segment0_base_dwords != 0x5000
        || target.gc_segment1_base_dwords != 0x7000
        || target.target_dword_offset != 0x9040
        || target.target_byte_offset != 0x24100 {
        return Err("K14.C21 synthetic generated BASE_IDX=1 resolver self-test failed");
    }
    if transaction_fingerprint(0x7550, 0xc0, target.target_dword_offset, 0x55aa55aa, 0x55aa55aa, 1) == 0 {
        return Err("K14.C21 transaction fingerprint self-test failed");
    }
    Ok(())
}

pub fn initialize(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<C21State, &'static str> {
    self_test()?;
    let c6 = native_gpu_c6::state();
    let c9 = native_gpu_c9::state();
    let c16 = native_gpu_c16::state();
    let c19 = native_gpu_c19::state();
    let c20 = native_gpu_c20::state();
    let mut s = C21State {
        amd_present: c9.amd_present,
        navi48: c9.profile == native_gpu_c9::ProfileId::Navi48Rx9070,
        profile_verified: c9.profile_verified && c9.pci_identity_consistent,
        exact_domain_live: c6.persistent_domain_live,
        c16_target_reviewed: c16.target == native_gpu_c16::ReviewedTarget::Gfx12GcScratchReg0 && c16.target_reviewed,
        c19_snapshot_verified: c19.live_snapshot_verified && c19.snapshot_fingerprint != 0,
        c20_exact_bases_ready: c20.c16_promotion_input_ready,
        source_register_imported: true,
        source_base_index_imported: true,
        snapshot_fingerprint: c19.snapshot_fingerprint,
        device_id: c9.device_id,
        revision: c9.revision,
        ..C21State::EMPTY
    };

    serial::println(format_args!(
        "[C21RV] reviewed GFX12 target rebind: SCRATCH_REG0={:#06x} BASE_IDX={} generated=true formula=GC_base[1]+reg C16_semantics={} guessed_register=false guessed_base=false",
        GFX12_SCRATCH_REG0_DWORD, GFX12_SCRATCH_REG0_BASE_IDX, s.c16_target_reviewed
    ));
    serial::println(format_args!(
        "[C21PG] post-discovery identity-write policy: require=verified_Navi48_profile+translated_domain+C16_reviewed_semantics+C19_checksum_snapshot+C20_GFX12_exact_bases+GC_base1+BAR5+memory_decode_on+bus_master_off; max_polls={} max_writes={} arbitrary=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master_enable=false",
        RADEON_C21_MAX_READBACK_POLLS, RADEON_C21_MAX_MMIO_WRITES
    ));
    serial::println(format_args!(
        "[C21TX] identity transaction contract: read_current_u32 -> reject_all_ones -> write_same_u32 -> exact_readback<=32 -> one_restore_on_failure -> recheck_memory_decode_and_bus_master"
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C21HW] GFX12 SCRATCH_REG0 identity-write: present=false qemu_deferred=true target=false attempted=false verified=false fallback=true"
        ));
    } else {
        let post_discovery_ready = s.navi48 && s.profile_verified && s.exact_domain_live
            && s.c16_target_reviewed && s.c19_snapshot_verified && s.c20_exact_bases_ready
            && c20.snapshot_fingerprint == c19.snapshot_fingerprint;
        if !post_discovery_ready {
            serial::println(format_args!(
                "[C21HW] GFX12 SCRATCH_REG0 identity-write: present=true devid={:#06x} navi48={} profile={} domain={} C16_reviewed={} C19_verified={} C20_ready={} attempted=false reason=post_discovery_gate_not_ready fallback=true",
                s.device_id, s.navi48, s.profile_verified, s.exact_domain_live,
                s.c16_target_reviewed, s.c19_snapshot_verified, s.c20_exact_bases_ready
            ));
        } else {
            let target = native_gpu_c19::with_verified_snapshot(resolve_gfx12_scratch_reg0)
                .ok_or("K14.C21 C19 verified snapshot unavailable")??;
            s.gc_segment1_resolved = target.gc_segment1_base_dwords != 0;
            s.gc_segment0_crosschecked = target.gc_segment0_base_dwords == c20.gc_base_dwords
                && target.gc_major == c20.gc_major && target.gc_minor == c20.gc_minor
                && target.gc_revision == c20.gc_revision;
            if !s.gc_segment0_crosschecked {
                return Err("K14.C21 resolved target disagrees with frozen C20 live GC proof");
            }
            s.gc_segment1_base_dwords = target.gc_segment1_base_dwords;
            s.target_dword_offset = target.target_dword_offset;
            s.target_byte_offset = target.target_byte_offset;

            let function = selected_function();
            let command_before = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_before_on = command_before & (1 << 1) != 0;
            s.bus_master_before_off = command_before & (1 << 2) == 0;
            if !s.memory_decode_before_on {
                return Err("K14.C21 Radeon PCI memory decoding is disabled");
            }
            if !s.bus_master_before_off {
                return Err("K14.C21 Radeon bus mastering unexpectedly enabled before identity write");
            }
            let bar = pci::memory_bar_base(function, RADEON_C21_MMIO_BAR_INDEX)
                .ok_or("K14.C21 Radeon BAR5 unavailable")?;
            s.bar5_ready = true;
            s.transaction_eligible = true;

            let page_off = s.target_byte_offset & !(RADEON_C21_PAGE_BYTES - 1);
            let in_page = s.target_byte_offset & (RADEON_C21_PAGE_BYTES - 1);
            if in_page + 4 > RADEON_C21_PAGE_BYTES {
                return Err("K14.C21 reviewed target crosses MMIO page boundary");
            }
            let phys = bar.checked_add(page_off).ok_or("K14.C21 MMIO physical overflow")?;
            let virt = paging::map_kernel_mmio(allocator, kernel_cr3, phys, RADEON_C21_PAGE_BYTES)?;
            let p = (virt + in_page) as *mut u32;

            s.value_before = unsafe { core::ptr::read_volatile(p) };
            if s.value_before == u32::MAX {
                return Err("K14.C21 reviewed SCRATCH_REG0 read returned all ones");
            }
            s.transaction_attempted = true;
            unsafe { core::ptr::write_volatile(p, s.value_before) };
            s.writes_performed = 1;
            while s.readback_polls < RADEON_C21_MAX_READBACK_POLLS {
                s.value_after = unsafe { core::ptr::read_volatile(p) };
                s.readback_polls += 1;
                if s.value_after == s.value_before {
                    s.transaction_verified = true;
                    break;
                }
            }
            if !s.transaction_verified {
                s.rollback_attempted = true;
                unsafe { core::ptr::write_volatile(p, s.value_before) };
                s.writes_performed = 2;
                let restored = unsafe { core::ptr::read_volatile(p) };
                s.rollback_verified = restored == s.value_before;
                s.value_after = restored;
                if !s.rollback_verified {
                    return Err("K14.C21 identity-write readback and bounded restore both failed");
                }
                return Err("K14.C21 identity-write required restore; transaction not qualified");
            }

            let command_after = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_after_on = command_after & (1 << 1) != 0;
            s.bus_master_after_off = command_after & (1 << 2) == 0;
            if !s.memory_decode_after_on || !s.bus_master_after_off {
                return Err("K14.C21 PCI decode/bus-master safety changed during identity write");
            }
            s.transaction_fingerprint = transaction_fingerprint(
                s.device_id, s.revision, s.target_dword_offset,
                s.value_before, s.value_after, s.writes_performed,
            );
            serial::println(format_args!(
                "[C21HW] GFX12 SCRATCH_REG0 identity-write: present=true navi48=true devid={:#06x} GC_base0={:#x} GC_base1={:#x} target_dwords={:#x} target_bytes={:#x} before={:#010x} after={:#010x} writes={} polls={} attempted=true verified=true memory_decode_before=true memory_decode_after=true bus_master_before=false bus_master_after=false fingerprint={:#018x} fallback=true",
                s.device_id, target.gc_segment0_base_dwords, target.gc_segment1_base_dwords,
                s.target_dword_offset, s.target_byte_offset, s.value_before, s.value_after,
                s.writes_performed, s.readback_polls, s.transaction_fingerprint
            ));
        }
    }

    if s.transaction_verified && (!s.transaction_eligible || !s.transaction_attempted
        || !s.gc_segment1_resolved || !s.gc_segment0_crosschecked || !s.bar5_ready
        || !s.memory_decode_before_on || !s.memory_decode_after_on
        || !s.bus_master_before_off || !s.bus_master_after_off) {
        return Err("K14.C21 transaction qualified without every post-discovery safety gate");
    }
    if s.writes_performed > RADEON_C21_MAX_MMIO_WRITES
        || s.arbitrary_mmio_write_enabled || s.mm_index_fallback_used || s.bar_resize_used
        || s.firmware_upload_enabled || s.command_submit_enabled || s.radeon_bus_master_enabled
        || RADEON_C21_ARBITRARY_MMIO_WRITES_ALLOWED || RADEON_C21_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C21_BAR_RESIZE_ALLOWED || RADEON_C21_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C21_COMMAND_SUBMIT_ALLOWED || RADEON_C21_BUS_MASTER_ALLOWED
        || RADEON_C21_GUESSED_REGISTER_ALLOWED || RADEON_C21_GUESSED_BASE_ALLOWED {
        return Err("K14.C21 destructive capability promoted outside reviewed identity-write contract");
    }

    serial::println(format_args!(
        "[C21RD] K14.C21 reviewed-target rebind ready: amd_present={} navi48={} profile={} domain={} C16_reviewed={} C19_verified={} C20_ready={} register={} base_idx={} GC_base1={} crosscheck={} target_dwords={:#x} BAR5={} memdecode_before={} eligible={} attempted={} verified={} writes={} polls={} memdecode_after={} bus_master_before_off={} bus_master_after_off={} snapshot_fp={:#018x} tx_fp={:#018x} arbitrary=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master=false fallback=true",
        s.amd_present, s.navi48, s.profile_verified, s.exact_domain_live,
        s.c16_target_reviewed, s.c19_snapshot_verified, s.c20_exact_bases_ready,
        s.source_register_imported, GFX12_SCRATCH_REG0_BASE_IDX,
        s.gc_segment1_resolved, s.gc_segment0_crosschecked, s.target_dword_offset,
        s.bar5_ready, s.memory_decode_before_on, s.transaction_eligible,
        s.transaction_attempted, s.transaction_verified, s.writes_performed,
        s.readback_polls, s.memory_decode_after_on, s.bus_master_before_off,
        s.bus_master_after_off, s.snapshot_fingerprint, s.transaction_fingerprint
    ));
    *STATE.lock() = s;
    Ok(s)
}

pub fn state() -> C21State { *STATE.lock() }

pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.device_id) << 40)
        | (u64::from(s.revision) << 32)
        | (u64::from(s.writes_performed) << 24)
        | (u64::from(s.readback_polls) << 16);
    for (bit, on) in [
        s.amd_present,                 // bit 0
        s.navi48,                      // bit 1
        s.profile_verified,            // bit 2
        s.exact_domain_live,           // bit 3
        s.c16_target_reviewed,         // bit 4
        s.c19_snapshot_verified,       // bit 5
        s.c20_exact_bases_ready,       // bit 6
        s.gc_segment1_resolved,        // bit 7
        s.gc_segment0_crosschecked,    // bit 8
        s.transaction_eligible,        // bit 9
        s.transaction_verified,        // bit 10
        s.bus_master_before_off,       // bit 11
        s.bus_master_after_off,        // bit 12
        s.fallback_armed,              // bit 13
    ].into_iter().enumerate() {
        if on { v |= 1u64 << bit; }
    }
    v
}
