//! K14.C26 final reviewed GFX12 MMIO allowlist and SCRATCH_REG1 read-proof gate.
//!
//! Frozen C25 exhaustively exercises the exact checksum-backed GFX12
//! SCRATCH_REG0 target through bounded reversible mutation/readback/restore
//! cycles. C26 closes K14 rather than opening another write milestone. It adds
//! one second AMD-generated scratch-class register, SCRATCH_REG1, resolves it
//! through the same checksum-qualified GC base slot used by C21-C25, proves the
//! new target is distinct and adjacent to SCRATCH_REG0, and permits bounded
//! reads only. No C26 path writes SCRATCH_REG1.
//!
//! C26 also materializes the reviewed two-entry K14 MMIO allowlist:
//!   SCRATCH_REG0 -> frozen reversible-probe authority inherited from C25
//!   SCRATCH_REG1 -> read-only authority
//!
//! Caller-selected addresses/values, arbitrary MMIO writes, MM_INDEX/MM_DATA,
//! BAR resizing, firmware upload, command submission, and Radeon bus-master
//! enable remain forbidden. A successful C26 runtime qualification is the K14
//! completion point; broader Radeon driver bring-up belongs to K15.

use crate::{
    memory::FrameAllocator,
    native_gpu_binding,
    native_gpu_c19,
    native_gpu_c21,
    native_gpu_c25,
    paging,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C26_ABI_VERSION: u32 = 1;

// AMD-generated GFX12 definitions from gc_12_0_0_offset.h.
pub const GFX12_SCRATCH_REG1_DWORD: u32 = 0x2041;
pub const GFX12_SCRATCH_REG1_BASE_IDX: u8 = 1;

pub const RADEON_C26_MMIO_BAR_INDEX: u8 = native_gpu_c25::RADEON_C25_MMIO_BAR_INDEX;
pub const RADEON_C26_PAGE_BYTES: u64 = native_gpu_c25::RADEON_C25_PAGE_BYTES;
pub const RADEON_C26_MAX_READ_SAMPLES: u8 = 4;
pub const RADEON_C26_MAX_MMIO_WRITES: u8 = 0;
pub const RADEON_C26_ALLOWLIST_ENTRY_COUNT: u8 = 2;
pub const RADEON_C26_REG1_READ_ALLOWED: bool = true;
pub const RADEON_C26_REG1_WRITE_ALLOWED: bool = false;
pub const RADEON_C26_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C26_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false;
pub const RADEON_C26_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false;
pub const RADEON_C26_MM_INDEX_FALLBACK_ALLOWED: bool = false;
pub const RADEON_C26_BAR_RESIZE_ALLOWED: bool = false;
pub const RADEON_C26_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C26_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C26_BUS_MASTER_ALLOWED: bool = false;
pub const K14C26_FINAL_K14_COMPLETION_GATE: bool = true;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewedAccess {
    FrozenReversibleProbe,
    ReadOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewedMmioEntry {
    pub dword: u32,
    pub base_idx: u8,
    pub access: ReviewedAccess,
}

pub const GFX12_REVIEWED_MMIO_ALLOWLIST: [ReviewedMmioEntry; 2] = [
    ReviewedMmioEntry {
        dword: native_gpu_c21::GFX12_SCRATCH_REG0_DWORD,
        base_idx: native_gpu_c21::GFX12_SCRATCH_REG0_BASE_IDX,
        access: ReviewedAccess::FrozenReversibleProbe,
    },
    ReviewedMmioEntry {
        dword: GFX12_SCRATCH_REG1_DWORD,
        base_idx: GFX12_SCRATCH_REG1_BASE_IDX,
        access: ReviewedAccess::ReadOnly,
    },
];

#[derive(Clone, Copy, Debug)]
pub struct C26State {
    pub amd_present: bool,
    pub navi48: bool,
    pub profile_verified: bool,
    pub exact_domain_live: bool,
    pub c19_snapshot_verified: bool,
    pub c20_exact_bases_ready: bool,
    pub c21_identity_verified: bool,
    pub c25_dual_pattern_verified: bool,
    pub c25_target_revalidated: bool,
    pub c25_transaction_fingerprint: u64,
    pub source_reg1_imported: bool,
    pub source_base_index_imported: bool,
    pub same_gc_base1: bool,
    pub reg1_resolved: bool,
    pub targets_distinct: bool,
    pub targets_adjacent: bool,
    pub allowlist_exact: bool,
    pub reg0_target_dword_offset: u64,
    pub reg0_target_byte_offset: u64,
    pub reg1_target_dword_offset: u64,
    pub reg1_target_byte_offset: u64,
    pub gc_segment1_base_dwords: u64,
    pub bar5_ready: bool,
    pub memory_decode_before_on: bool,
    pub memory_decode_after_on: bool,
    pub bus_master_before_off: bool,
    pub bus_master_after_off: bool,
    pub read_eligible: bool,
    pub read_attempted: bool,
    pub read_samples: u8,
    pub read_samples_valid: u8,
    pub first_value: u32,
    pub last_value: u32,
    pub read_proof_valid: bool,
    pub writes_performed: u8,
    pub no_write_verified: bool,
    pub k14_completion_verified: bool,
    pub snapshot_fingerprint: u64,
    pub read_fingerprint: u64,
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

impl C26State {
    pub const EMPTY: Self = Self {
        amd_present: false, navi48: false, profile_verified: false,
        exact_domain_live: false, c19_snapshot_verified: false,
        c20_exact_bases_ready: false, c21_identity_verified: false,
        c25_dual_pattern_verified: false, c25_target_revalidated: false,
        c25_transaction_fingerprint: 0, source_reg1_imported: false,
        source_base_index_imported: false, same_gc_base1: false,
        reg1_resolved: false, targets_distinct: false, targets_adjacent: false,
        allowlist_exact: false, reg0_target_dword_offset: 0,
        reg0_target_byte_offset: 0, reg1_target_dword_offset: 0,
        reg1_target_byte_offset: 0, gc_segment1_base_dwords: 0,
        bar5_ready: false, memory_decode_before_on: false,
        memory_decode_after_on: false, bus_master_before_off: false,
        bus_master_after_off: false, read_eligible: false,
        read_attempted: false, read_samples: 0, read_samples_valid: 0,
        first_value: 0, last_value: 0, read_proof_valid: false,
        writes_performed: 0, no_write_verified: true,
        k14_completion_verified: false, snapshot_fingerprint: 0,
        read_fingerprint: 0, arbitrary_mmio_write_enabled: false,
        mm_index_fallback_used: false, bar_resize_used: false,
        firmware_upload_enabled: false, command_submit_enabled: false,
        radeon_bus_master_enabled: false, fallback_armed: true,
        device_id: 0, revision: 0,
    };
}

static STATE: SpinLock<C26State> = SpinLock::new(C26State::EMPTY);

/// Resolve SCRATCH_REG1 from the exact same verified GFX12 GC base-slot proof
/// used by frozen C21-C25. Reusing the C21 parser also reuses its C20 GC-version
/// and slot-0 cross-check; only the AMD-generated register dword changes.
pub fn resolve_gfx12_scratch_reg1(
    bytes: &[u8],
) -> Result<native_gpu_c21::ReviewedTarget, &'static str> {
    let reg0 = native_gpu_c21::resolve_gfx12_scratch_reg0(bytes)?;
    if !reg0.valid || reg0.gc_major != 12 || reg0.gc_segment1_base_dwords == 0 {
        return Err("K14.C26 frozen C21 GFX12 base-slot proof is unavailable");
    }
    if GFX12_SCRATCH_REG1_BASE_IDX != native_gpu_c21::GFX12_SCRATCH_REG0_BASE_IDX {
        return Err("K14.C26 SCRATCH_REG1 base index diverges from reviewed GFX12 scratch block");
    }
    let target_dword_offset = reg0.gc_segment1_base_dwords
        .checked_add(u64::from(GFX12_SCRATCH_REG1_DWORD))
        .ok_or("K14.C26 SCRATCH_REG1 dword offset overflow")?;
    let target_byte_offset = target_dword_offset
        .checked_mul(4)
        .ok_or("K14.C26 SCRATCH_REG1 byte offset overflow")?;
    Ok(native_gpu_c21::ReviewedTarget {
        valid: true,
        gc_major: reg0.gc_major,
        gc_minor: reg0.gc_minor,
        gc_revision: reg0.gc_revision,
        gc_segment0_base_dwords: reg0.gc_segment0_base_dwords,
        gc_segment1_base_dwords: reg0.gc_segment1_base_dwords,
        target_dword_offset,
        target_byte_offset,
    })
}

fn selected_function() -> pci::PciFunction {
    let b = native_gpu_binding::state();
    pci::PciFunction {
        bus: b.selected_bus, device: b.selected_device, function: b.selected_function,
        vendor_id: 0, device_id: 0, class_code: 0, subclass: 0,
        programming_interface: 0, revision: 0, header_type: 0,
    }
}

fn fingerprint(s: &C26State) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for byte in s.device_id.to_le_bytes().into_iter()
        .chain([s.revision, s.read_samples, s.read_samples_valid])
        .chain(s.reg0_target_dword_offset.to_le_bytes())
        .chain(s.reg1_target_dword_offset.to_le_bytes())
        .chain(s.first_value.to_le_bytes())
        .chain(s.last_value.to_le_bytes())
        .chain(s.c25_transaction_fingerprint.to_le_bytes()) {
        h ^= u64::from(byte);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn self_test() -> Result<(), &'static str> {
    if K14C26_ABI_VERSION != 1
        || GFX12_SCRATCH_REG1_DWORD != 0x2041
        || GFX12_SCRATCH_REG1_BASE_IDX != 1
        || GFX12_SCRATCH_REG1_DWORD != native_gpu_c21::GFX12_SCRATCH_REG0_DWORD + 1
        || GFX12_SCRATCH_REG1_BASE_IDX != native_gpu_c21::GFX12_SCRATCH_REG0_BASE_IDX
        || RADEON_C26_MMIO_BAR_INDEX != 5
        || RADEON_C26_PAGE_BYTES != 4096
        || RADEON_C26_MAX_READ_SAMPLES != 4
        || RADEON_C26_MAX_MMIO_WRITES != 0
        || RADEON_C26_ALLOWLIST_ENTRY_COUNT != 2
        || !RADEON_C26_REG1_READ_ALLOWED
        || RADEON_C26_REG1_WRITE_ALLOWED
        || RADEON_C26_ARBITRARY_MMIO_WRITES_ALLOWED
        || RADEON_C26_CALLER_SUPPLIED_VALUE_ALLOWED
        || RADEON_C26_CALLER_SUPPLIED_ADDRESS_ALLOWED
        || RADEON_C26_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C26_BAR_RESIZE_ALLOWED
        || RADEON_C26_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C26_COMMAND_SUBMIT_ALLOWED
        || RADEON_C26_BUS_MASTER_ALLOWED
        || !K14C26_FINAL_K14_COMPLETION_GATE {
        return Err("K14.C26 final allowlist/read-only constants invalid");
    }
    let reg0 = GFX12_REVIEWED_MMIO_ALLOWLIST[0];
    let reg1 = GFX12_REVIEWED_MMIO_ALLOWLIST[1];
    if reg0.dword != native_gpu_c21::GFX12_SCRATCH_REG0_DWORD
        || reg0.base_idx != 1
        || reg0.access != ReviewedAccess::FrozenReversibleProbe
        || reg1.dword != GFX12_SCRATCH_REG1_DWORD
        || reg1.base_idx != 1
        || reg1.access != ReviewedAccess::ReadOnly {
        return Err("K14.C26 reviewed MMIO allowlist self-test failed");
    }
    Ok(())
}

pub fn initialize(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<C26State, &'static str> {
    self_test()?;
    let c19 = native_gpu_c19::state();
    let c21 = native_gpu_c21::state();
    let c25 = native_gpu_c25::state();
    let mut s = C26State {
        amd_present: c25.amd_present,
        navi48: c25.navi48,
        profile_verified: c25.profile_verified,
        exact_domain_live: c25.exact_domain_live,
        c19_snapshot_verified: c25.c19_snapshot_verified,
        c20_exact_bases_ready: c25.c20_exact_bases_ready,
        c21_identity_verified: c25.c21_identity_verified,
        c25_dual_pattern_verified: c25.dual_pattern_verified,
        c25_target_revalidated: c25.target_revalidated,
        c25_transaction_fingerprint: c25.transaction_fingerprint,
        source_reg1_imported: true,
        source_base_index_imported: true,
        reg0_target_dword_offset: c25.target_dword_offset,
        reg0_target_byte_offset: c25.target_byte_offset,
        gc_segment1_base_dwords: c21.gc_segment1_base_dwords,
        snapshot_fingerprint: c25.snapshot_fingerprint,
        device_id: c25.device_id,
        revision: c25.revision,
        ..C26State::EMPTY
    };

    s.allowlist_exact = GFX12_REVIEWED_MMIO_ALLOWLIST[0].dword
            == native_gpu_c21::GFX12_SCRATCH_REG0_DWORD
        && GFX12_REVIEWED_MMIO_ALLOWLIST[0].base_idx == 1
        && GFX12_REVIEWED_MMIO_ALLOWLIST[1].dword == GFX12_SCRATCH_REG1_DWORD
        && GFX12_REVIEWED_MMIO_ALLOWLIST[1].base_idx == 1
        && GFX12_REVIEWED_MMIO_ALLOWLIST[1].access == ReviewedAccess::ReadOnly;

    serial::println(format_args!(
        "[C26RV] second reviewed GFX12 target: SCRATCH_REG1={:#06x} BASE_IDX={} generated=true formula=GC_base[1]+reg source_same_block=true write_authority=false",
        GFX12_SCRATCH_REG1_DWORD, GFX12_SCRATCH_REG1_BASE_IDX
    ));
    serial::println(format_args!(
        "[C26AL] final K14 MMIO allowlist: entries={} REG0=frozen_reversible_probe REG1=read_only caller_address=false caller_value=false REG1_write=false arbitrary=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master_enable=false",
        RADEON_C26_ALLOWLIST_ENTRY_COUNT
    ));
    serial::println(format_args!(
        "[C26PG] K14 completion policy: require=C25_dual_pattern+same_checksum_snapshot+same_GC_base1+REG0_REG1_distinct_adjacent+BAR5+memory_decode_on+bus_master_off max_reads={} max_writes={} REG1_read=true REG1_write=false",
        RADEON_C26_MAX_READ_SAMPLES, RADEON_C26_MAX_MMIO_WRITES
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C26HW] GFX12 SCRATCH_REG1 read proof: present=false qemu_deferred=true resolved=false attempted=false reads=0 writes=0 fallback=true"
        ));
    } else {
        let c25_ready = s.navi48 && s.profile_verified && s.exact_domain_live
            && s.c19_snapshot_verified && s.c20_exact_bases_ready
            && s.c21_identity_verified && s.c25_dual_pattern_verified
            && s.c25_target_revalidated && s.c25_transaction_fingerprint != 0
            && s.snapshot_fingerprint != 0
            && s.snapshot_fingerprint == c19.snapshot_fingerprint
            && c25.memory_decode_after_on && c25.bus_master_after_off;
        if !c25_ready {
            serial::println(format_args!(
                "[C26HW] GFX12 SCRATCH_REG1 read proof: present=true devid={:#06x} C25_dual={} C25_target={} snapshot_match={} attempted=false reason=C25_final_prerequisite_not_ready writes=0 fallback=true",
                s.device_id, s.c25_dual_pattern_verified, s.c25_target_revalidated,
                s.snapshot_fingerprint != 0 && s.snapshot_fingerprint == c19.snapshot_fingerprint
            ));
        } else {
            let reg0 = native_gpu_c19::with_verified_snapshot(native_gpu_c21::resolve_gfx12_scratch_reg0)
                .ok_or("K14.C26 C19 verified snapshot unavailable for REG0")??;
            let reg1 = native_gpu_c19::with_verified_snapshot(resolve_gfx12_scratch_reg1)
                .ok_or("K14.C26 C19 verified snapshot unavailable for REG1")??;

            s.same_gc_base1 = reg0.gc_segment1_base_dwords == reg1.gc_segment1_base_dwords
                && reg1.gc_segment1_base_dwords == s.gc_segment1_base_dwords;
            s.reg1_resolved = reg1.valid && reg1.gc_major == 12 && s.same_gc_base1;
            s.reg1_target_dword_offset = reg1.target_dword_offset;
            s.reg1_target_byte_offset = reg1.target_byte_offset;
            s.targets_distinct = reg1.target_dword_offset != reg0.target_dword_offset
                && reg1.target_byte_offset != reg0.target_byte_offset;
            s.targets_adjacent = reg1.target_dword_offset == reg0.target_dword_offset + 1
                && reg1.target_byte_offset == reg0.target_byte_offset + 4;

            if reg0.target_dword_offset != s.reg0_target_dword_offset
                || reg0.target_byte_offset != s.reg0_target_byte_offset {
                return Err("K14.C26 frozen C25 REG0 target changed across K14 completion boundary");
            }
            if !s.reg1_resolved || !s.targets_distinct || !s.targets_adjacent || !s.allowlist_exact {
                return Err("K14.C26 reviewed REG1/allowlist target proof failed");
            }

            let function = selected_function();
            let command_before = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_before_on = command_before & (1 << 1) != 0;
            s.bus_master_before_off = command_before & (1 << 2) == 0;
            if !s.memory_decode_before_on {
                return Err("K14.C26 Radeon PCI memory decoding is disabled");
            }
            if !s.bus_master_before_off {
                return Err("K14.C26 Radeon bus mastering unexpectedly enabled before REG1 read proof");
            }

            let bar = pci::memory_bar_base(function, RADEON_C26_MMIO_BAR_INDEX)
                .ok_or("K14.C26 Radeon BAR5 unavailable")?;
            s.bar5_ready = true;
            s.read_eligible = true;

            let page_off = s.reg1_target_byte_offset & !(RADEON_C26_PAGE_BYTES - 1);
            let in_page = s.reg1_target_byte_offset & (RADEON_C26_PAGE_BYTES - 1);
            if in_page & 3 != 0 || in_page + 4 > RADEON_C26_PAGE_BYTES {
                return Err("K14.C26 reviewed REG1 target is not aligned inside one MMIO page");
            }
            let phys = bar.checked_add(page_off).ok_or("K14.C26 MMIO physical overflow")?;
            let virt = paging::map_kernel_mmio(allocator, kernel_cr3, phys, RADEON_C26_PAGE_BYTES)?;
            let p = (virt + in_page) as *const u32;

            s.read_attempted = true;
            let mut i = 0u8;
            while i < RADEON_C26_MAX_READ_SAMPLES {
                let value = unsafe { core::ptr::read_volatile(p) };
                if i == 0 { s.first_value = value; }
                s.last_value = value;
                s.read_samples = s.read_samples.saturating_add(1);
                if value != u32::MAX {
                    s.read_samples_valid = s.read_samples_valid.saturating_add(1);
                }
                i += 1;
            }
            s.read_proof_valid = s.read_samples == RADEON_C26_MAX_READ_SAMPLES
                && s.read_samples_valid == RADEON_C26_MAX_READ_SAMPLES;
            if !s.read_proof_valid {
                return Err("K14.C26 SCRATCH_REG1 bounded read proof returned all-ones/open-bus sample");
            }

            let command_after = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_after_on = command_after & (1 << 1) != 0;
            s.bus_master_after_off = command_after & (1 << 2) == 0;
            if !s.memory_decode_after_on {
                return Err("K14.C26 Radeon PCI memory decoding changed during REG1 read proof");
            }
            if !s.bus_master_after_off {
                return Err("K14.C26 Radeon bus mastering became enabled during REG1 read proof");
            }

            s.no_write_verified = s.writes_performed == 0;
            s.read_fingerprint = fingerprint(&s);
            s.k14_completion_verified = s.reg1_resolved && s.targets_distinct
                && s.targets_adjacent && s.allowlist_exact && s.read_proof_valid
                && s.no_write_verified && s.memory_decode_before_on
                && s.memory_decode_after_on && s.bus_master_before_off
                && s.bus_master_after_off && s.read_fingerprint != 0;
            if !s.k14_completion_verified {
                return Err("K14.C26 final K14 MMIO completion proof did not close");
            }

            serial::println(format_args!(
                "[C26HW] GFX12 SCRATCH_REG1 read proof: present=true navi48=true devid={:#06x} GC_base1={:#x} REG0_dwords={:#x} REG1_dwords={:#x} REG1_bytes={:#x} distinct=true adjacent=true reads={} valid_reads={} first={:#010x} last={:#010x} writes=0 memory_decode_before=true memory_decode_after=true bus_master_before=false bus_master_after=false fingerprint={:#018x} fallback=true",
                s.device_id, s.gc_segment1_base_dwords, s.reg0_target_dword_offset,
                s.reg1_target_dword_offset, s.reg1_target_byte_offset,
                s.read_samples, s.read_samples_valid, s.first_value, s.last_value,
                s.read_fingerprint
            ));
        }
    }

    if s.writes_performed != 0
        || !s.no_write_verified
        || s.arbitrary_mmio_write_enabled
        || s.mm_index_fallback_used
        || s.bar_resize_used
        || s.firmware_upload_enabled
        || s.command_submit_enabled
        || s.radeon_bus_master_enabled {
        return Err("K14.C26 final K14 safety fence violated");
    }

    serial::println(format_args!(
        "[C26RD] K14.C26 completion ready: amd_present={} navi48={} profile={} domain={} C19_verified={} C20_ready={} C21_identity={} C25_dual={} C25_target={} source_REG1={} base_idx={} same_GC_base1={} REG1_resolved={} distinct={} adjacent={} allowlist={} REG0_dwords={:#x} REG1_dwords={:#x} BAR5={} eligible={} attempted={} reads={} valid_reads={} read_proof={} writes={} no_write={} memdecode_before={} memdecode_after={} bus_master_before_off={} bus_master_after_off={} completion={} c25_fp={:#018x} read_fp={:#018x} arbitrary=false caller_value=false caller_address=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master=false fallback=true",
        s.amd_present, s.navi48, s.profile_verified, s.exact_domain_live,
        s.c19_snapshot_verified, s.c20_exact_bases_ready, s.c21_identity_verified,
        s.c25_dual_pattern_verified, s.c25_target_revalidated, s.source_reg1_imported,
        GFX12_SCRATCH_REG1_BASE_IDX, s.same_gc_base1, s.reg1_resolved,
        s.targets_distinct, s.targets_adjacent, s.allowlist_exact,
        s.reg0_target_dword_offset, s.reg1_target_dword_offset, s.bar5_ready,
        s.read_eligible, s.read_attempted, s.read_samples, s.read_samples_valid,
        s.read_proof_valid, s.writes_performed, s.no_write_verified,
        s.memory_decode_before_on, s.memory_decode_after_on,
        s.bus_master_before_off, s.bus_master_after_off, s.k14_completion_verified,
        s.c25_transaction_fingerprint, s.read_fingerprint
    ));

    *STATE.lock() = s;
    Ok(s)
}

pub fn state() -> C26State { *STATE.lock() }

pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.device_id) << 40)
        | (u64::from(s.revision) << 32)
        | (u64::from(s.read_samples) << 24);
    for (bit, on) in [
        s.amd_present,                 // bit 0
        s.navi48,                      // bit 1
        s.c25_dual_pattern_verified,   // bit 2
        s.reg1_resolved,               // bit 3
        s.targets_distinct,            // bit 4
        s.targets_adjacent,            // bit 5
        s.allowlist_exact,              // bit 6
        s.read_eligible,               // bit 7
        s.read_attempted,              // bit 8
        s.read_proof_valid,             // bit 9
        s.no_write_verified,            // bit 10
        s.bus_master_after_off,         // bit 11
        s.memory_decode_after_on,       // bit 12
        s.k14_completion_verified,      // bit 13
        s.fallback_armed,               // bit 14
    ].into_iter().enumerate() {
        if on { v |= 1u64 << bit; }
    }
    v
}
