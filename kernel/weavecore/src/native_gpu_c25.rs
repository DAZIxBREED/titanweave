//! K14.C25 dual multi-bit GFX12 SCRATCH_REG0 pattern-stability gate.
//!
//! Frozen C24 proves one deterministic reversible four-bit mutation of the
//! exact checksum-backed GFX12 SCRATCH_REG0 target with exact restoration.
//! C25 does not widen the writable register set. It first requires the C24-
//! restored value to persist, then executes two distinct internally-derived
//! four-bit pattern/readback/restore cycles on the same target, with an
//! explicit inter-cycle persistence check. One restore retry per cycle exists
//! only to recover the original scratch value.
//!
//! Caller-selected addresses/values, arbitrary MMIO writes, MM_INDEX/MM_DATA,
//! BAR resizing, firmware upload, command submission and Radeon bus-master
//! enable remain forbidden.

use crate::{
    memory::FrameAllocator,
    native_gpu_binding,
    native_gpu_c19,
    native_gpu_c21,
    native_gpu_c24,
    paging,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C25_ABI_VERSION: u32 = 1;
pub const RADEON_C25_MMIO_BAR_INDEX: u8 = native_gpu_c24::RADEON_C24_MMIO_BAR_INDEX;
pub const RADEON_C25_PAGE_BYTES: u64 = native_gpu_c24::RADEON_C24_PAGE_BYTES;
pub const RADEON_C25_MAX_PATTERN_POLLS_PER_CYCLE: u8 = 32;
pub const RADEON_C25_MAX_RESTORE_POLLS_PER_CYCLE: u8 = 32;
pub const RADEON_C25_MAX_MMIO_WRITES: u8 = 6; // (pattern + restore + recovery retry) * 2
pub const RADEON_C25_PATTERN_BITS_PER_CYCLE: u32 = 4;
pub const RADEON_C25_DUAL_MULTI_BIT_PATTERN_ALLOWED: bool = true;
pub const RADEON_C25_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C25_MM_INDEX_FALLBACK_ALLOWED: bool = false;
pub const RADEON_C25_BAR_RESIZE_ALLOWED: bool = false;
pub const RADEON_C25_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C25_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C25_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C25_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false;
pub const RADEON_C25_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct C25State {
    pub amd_present: bool,
    pub navi48: bool,
    pub profile_verified: bool,
    pub exact_domain_live: bool,
    pub c19_snapshot_verified: bool,
    pub c20_exact_bases_ready: bool,
    pub c21_identity_verified: bool,
    pub c22_mutation_verified: bool,
    pub c22_restore_verified: bool,
    pub c23_dual_cycle_verified: bool,
    pub c24_pattern_verified: bool,
    pub c24_restore_verified: bool,
    pub c24_target_revalidated: bool,
    pub c24_transaction_fingerprint: u64,
    pub target_revalidated: bool,
    pub c24_restore_persisted: bool,
    pub intercycle_restore_persisted: bool,
    pub target_dword_offset: u64,
    pub target_byte_offset: u64,
    pub bar5_ready: bool,
    pub memory_decode_before_on: bool,
    pub memory_decode_after_on: bool,
    pub bus_master_before_off: bool,
    pub bus_master_after_off: bool,
    pub transaction_eligible: bool,
    pub value_expected: u32,
    pub persistence_observed: u32,
    pub intercycle_observed: u32,
    pub pattern_a: u32,
    pub pattern_b: u32,
    pub cycle_a_attempted: bool,
    pub cycle_a_pattern_verified: bool,
    pub cycle_a_restore_verified: bool,
    pub cycle_a_restore_retry_used: bool,
    pub cycle_b_attempted: bool,
    pub cycle_b_pattern_verified: bool,
    pub cycle_b_restore_verified: bool,
    pub cycle_b_restore_retry_used: bool,
    pub cycle_a_pattern_polls: u8,
    pub cycle_a_restore_polls: u8,
    pub cycle_b_pattern_polls: u8,
    pub cycle_b_restore_polls: u8,
    pub writes_performed: u8,
    pub dual_pattern_verified: bool,
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

impl C25State {
    pub const EMPTY: Self = Self {
        amd_present: false, navi48: false, profile_verified: false,
        exact_domain_live: false, c19_snapshot_verified: false,
        c20_exact_bases_ready: false, c21_identity_verified: false,
        c22_mutation_verified: false, c22_restore_verified: false,
        c23_dual_cycle_verified: false, c24_pattern_verified: false,
        c24_restore_verified: false, c24_target_revalidated: false,
        c24_transaction_fingerprint: 0, target_revalidated: false,
        c24_restore_persisted: false, intercycle_restore_persisted: false,
        target_dword_offset: 0, target_byte_offset: 0, bar5_ready: false,
        memory_decode_before_on: false, memory_decode_after_on: false,
        bus_master_before_off: false, bus_master_after_off: false,
        transaction_eligible: false, value_expected: 0,
        persistence_observed: 0, intercycle_observed: 0,
        pattern_a: 0, pattern_b: 0, cycle_a_attempted: false,
        cycle_a_pattern_verified: false, cycle_a_restore_verified: false,
        cycle_a_restore_retry_used: false, cycle_b_attempted: false,
        cycle_b_pattern_verified: false, cycle_b_restore_verified: false,
        cycle_b_restore_retry_used: false, cycle_a_pattern_polls: 0,
        cycle_a_restore_polls: 0, cycle_b_pattern_polls: 0,
        cycle_b_restore_polls: 0, writes_performed: 0,
        dual_pattern_verified: false, snapshot_fingerprint: 0,
        transaction_fingerprint: 0, arbitrary_mmio_write_enabled: false,
        mm_index_fallback_used: false, bar_resize_used: false,
        firmware_upload_enabled: false, command_submit_enabled: false,
        radeon_bus_master_enabled: false, fallback_armed: true,
        device_id: 0, revision: 0,
    };
}

#[derive(Clone, Copy)]
struct CycleResult {
    pattern_verified: bool,
    restore_verified: bool,
    restore_retry_used: bool,
    writes: u8,
    pattern_polls: u8,
    restore_polls: u8,
}

static STATE: SpinLock<C25State> = SpinLock::new(C25State::EMPTY);

/// Cycle A normally toggles bits 0..3. If that exact result would be all ones,
/// use bits 8..11 instead. Exactly four bits always change.
pub const fn derive_pattern_a(original: u32) -> u32 {
    let low = original ^ 0x0000_000f;
    if low != u32::MAX { low } else { original ^ 0x0000_0f00 }
}

/// Cycle B normally toggles bits 4..7. If that exact result would be all ones,
/// use bits 12..15 instead. The candidate mask is always distinct from cycle A.
pub const fn derive_pattern_b(original: u32) -> u32 {
    let high = original ^ 0x0000_00f0;
    if high != u32::MAX { high } else { original ^ 0x0000_f000 }
}

fn selected_function() -> pci::PciFunction {
    let b = native_gpu_binding::state();
    pci::PciFunction {
        bus: b.selected_bus, device: b.selected_device, function: b.selected_function,
        vendor_id: 0, device_id: 0, class_code: 0, subclass: 0,
        programming_interface: 0, revision: 0, header_type: 0,
    }
}

fn poll_exact(p: *mut u32, expected: u32, limit: u8) -> (bool, u8) {
    let mut polls = 0u8;
    while polls < limit {
        let observed = unsafe { core::ptr::read_volatile(p) };
        polls += 1;
        if observed == expected { return (true, polls); }
    }
    (false, polls)
}

fn run_cycle(p: *mut u32, original: u32, pattern: u32) -> CycleResult {
    unsafe { core::ptr::write_volatile(p, pattern) };
    let (pattern_verified, pattern_polls) = poll_exact(
        p, pattern, RADEON_C25_MAX_PATTERN_POLLS_PER_CYCLE
    );

    // Restoration is mandatory even if pattern readback failed.
    unsafe { core::ptr::write_volatile(p, original) };
    let (mut restore_verified, mut restore_polls) = poll_exact(
        p, original, RADEON_C25_MAX_RESTORE_POLLS_PER_CYCLE
    );
    let mut restore_retry_used = false;
    let mut writes = 2u8;

    if !restore_verified {
        restore_retry_used = true;
        unsafe { core::ptr::write_volatile(p, original) };
        writes = 3;
        let (retry_ok, retry_polls) = poll_exact(
            p, original, RADEON_C25_MAX_RESTORE_POLLS_PER_CYCLE
        );
        restore_polls = restore_polls.saturating_add(retry_polls);
        restore_verified = retry_ok;
    }

    CycleResult {
        pattern_verified, restore_verified, restore_retry_used, writes,
        pattern_polls, restore_polls,
    }
}

fn fingerprint(s: &C25State) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for byte in s.device_id.to_le_bytes().into_iter()
        .chain([s.revision])
        .chain(s.target_dword_offset.to_le_bytes())
        .chain(s.value_expected.to_le_bytes())
        .chain(s.pattern_a.to_le_bytes())
        .chain(s.pattern_b.to_le_bytes())
        .chain([s.writes_performed, s.cycle_a_pattern_polls, s.cycle_a_restore_polls,
                s.cycle_b_pattern_polls, s.cycle_b_restore_polls])
        .chain(s.c24_transaction_fingerprint.to_le_bytes()) {
        h ^= u64::from(byte);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn self_test() -> Result<(), &'static str> {
    if K14C25_ABI_VERSION != 1
        || RADEON_C25_MMIO_BAR_INDEX != 5
        || RADEON_C25_PAGE_BYTES != 4096
        || RADEON_C25_MAX_PATTERN_POLLS_PER_CYCLE != 32
        || RADEON_C25_MAX_RESTORE_POLLS_PER_CYCLE != 32
        || RADEON_C25_MAX_MMIO_WRITES != 6
        || RADEON_C25_PATTERN_BITS_PER_CYCLE != 4
        || !RADEON_C25_DUAL_MULTI_BIT_PATTERN_ALLOWED
        || RADEON_C25_ARBITRARY_MMIO_WRITES_ALLOWED
        || RADEON_C25_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C25_BAR_RESIZE_ALLOWED
        || RADEON_C25_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C25_COMMAND_SUBMIT_ALLOWED
        || RADEON_C25_BUS_MASTER_ALLOWED
        || RADEON_C25_CALLER_SUPPLIED_VALUE_ALLOWED
        || RADEON_C25_CALLER_SUPPLIED_ADDRESS_ALLOWED {
        return Err("K14.C25 dual-pattern/fail-closed constants invalid");
    }
    for original in [0u32, 1, 0x0f, 0xf0, 0xf00, 0xf000, 0x55aa_55aa,
                     0xffff_fff0, 0xffff_ff0f, 0xffff_f0ff, 0xffff_0fff,
                     0x8000_0000, 0xffff_fffe] {
        let a = derive_pattern_a(original);
        let b = derive_pattern_b(original);
        if a == original || b == original || a == b || a == u32::MAX || b == u32::MAX
            || (original ^ a).count_ones() != RADEON_C25_PATTERN_BITS_PER_CYCLE
            || (original ^ b).count_ones() != RADEON_C25_PATTERN_BITS_PER_CYCLE {
            return Err("K14.C25 distinct four-bit pattern self-test failed");
        }
    }
    Ok(())
}

pub fn initialize(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<C25State, &'static str> {
    self_test()?;
    let c19 = native_gpu_c19::state();
    let c21 = native_gpu_c21::state();
    let c24 = native_gpu_c24::state();
    let mut s = C25State {
        amd_present: c24.amd_present,
        navi48: c24.navi48,
        profile_verified: c24.profile_verified,
        exact_domain_live: c24.exact_domain_live,
        c19_snapshot_verified: c24.c19_snapshot_verified,
        c20_exact_bases_ready: c24.c20_exact_bases_ready,
        c21_identity_verified: c24.c21_identity_verified,
        c22_mutation_verified: c24.c22_mutation_verified,
        c22_restore_verified: c24.c22_restore_verified,
        c23_dual_cycle_verified: c24.c23_dual_cycle_verified,
        c24_pattern_verified: c24.pattern_verified,
        c24_restore_verified: c24.restore_verified,
        c24_target_revalidated: c24.target_revalidated,
        c24_transaction_fingerprint: c24.transaction_fingerprint,
        target_dword_offset: c24.target_dword_offset,
        target_byte_offset: c24.target_byte_offset,
        value_expected: c24.value_expected,
        snapshot_fingerprint: c24.snapshot_fingerprint,
        device_id: c24.device_id,
        revision: c24.revision,
        ..C25State::EMPTY
    };

    serial::println(format_args!(
        "[C25DP] dual multi-bit patterns: source=C24_exact_restore patternA=XOR_low_nibble_or_bits8_11 patternB=XOR_high_nibble_or_bits12_15 changed_bits_each=4 distinct=true caller_value=false caller_address=false"
    ));
    serial::println(format_args!(
        "[C25PG] dual-pattern policy: require=C24_pattern_verified+C24_restore_verified+C24_target_revalidated+same_checksum_snapshot+same_exact_target+BAR5+memory_decode_on+bus_master_off; pattern_polls_per_cycle<={} restore_polls_per_cycle<={} max_writes={} arbitrary=false caller_value=false caller_address=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master_enable=false",
        RADEON_C25_MAX_PATTERN_POLLS_PER_CYCLE,
        RADEON_C25_MAX_RESTORE_POLLS_PER_CYCLE,
        RADEON_C25_MAX_MMIO_WRITES
    ));
    serial::println(format_args!(
        "[C25TX] dual-pattern stability contract: verify_C24_restored_value -> pattern_A -> exact_A -> restore -> exact_restore_A -> verify_intercycle_persistence -> distinct_pattern_B -> exact_B -> restore -> exact_restore_B -> recheck_memory_decode_and_bus_master"
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C25HW] GFX12 SCRATCH_REG0 dual multi-bit pattern stability: present=false qemu_deferred=true persistence=false cycleA=false cycleB=false fallback=true"
        ));
    } else {
        let c24_ready = s.navi48 && s.profile_verified && s.exact_domain_live
            && s.c19_snapshot_verified && s.c20_exact_bases_ready
            && s.c21_identity_verified && s.c22_mutation_verified
            && s.c22_restore_verified && s.c23_dual_cycle_verified
            && s.c24_pattern_verified && s.c24_restore_verified
            && s.c24_target_revalidated && s.c24_transaction_fingerprint != 0
            && s.snapshot_fingerprint != 0
            && s.snapshot_fingerprint == c19.snapshot_fingerprint;
        if !c24_ready {
            serial::println(format_args!(
                "[C25HW] GFX12 SCRATCH_REG0 dual multi-bit pattern stability: present=true devid={:#06x} C24_pattern={} C24_restore={} C24_target={} snapshot_match={} attempted=false reason=C24_dual_pattern_gate_not_ready fallback=true",
                s.device_id, s.c24_pattern_verified, s.c24_restore_verified,
                s.c24_target_revalidated,
                s.snapshot_fingerprint != 0 && s.snapshot_fingerprint == c19.snapshot_fingerprint
            ));
        } else {
            let target = native_gpu_c19::with_verified_snapshot(native_gpu_c21::resolve_gfx12_scratch_reg0)
                .ok_or("K14.C25 C19 verified snapshot unavailable")??;
            s.target_revalidated = target.valid
                && target.target_dword_offset == s.target_dword_offset
                && target.target_byte_offset == s.target_byte_offset
                && target.gc_segment1_base_dwords == c21.gc_segment1_base_dwords;
            if !s.target_revalidated {
                return Err("K14.C25 live target no longer matches frozen C24 proof");
            }

            let function = selected_function();
            let command_before = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_before_on = command_before & (1 << 1) != 0;
            s.bus_master_before_off = command_before & (1 << 2) == 0;
            if !s.memory_decode_before_on {
                return Err("K14.C25 Radeon PCI memory decoding is disabled");
            }
            if !s.bus_master_before_off {
                return Err("K14.C25 Radeon bus mastering unexpectedly enabled before dual-pattern test");
            }

            let bar = pci::memory_bar_base(function, RADEON_C25_MMIO_BAR_INDEX)
                .ok_or("K14.C25 Radeon BAR5 unavailable")?;
            s.bar5_ready = true;
            s.transaction_eligible = true;

            let page_off = s.target_byte_offset & !(RADEON_C25_PAGE_BYTES - 1);
            let in_page = s.target_byte_offset & (RADEON_C25_PAGE_BYTES - 1);
            if in_page + 4 > RADEON_C25_PAGE_BYTES {
                return Err("K14.C25 reviewed target crosses MMIO page boundary");
            }
            let phys = bar.checked_add(page_off).ok_or("K14.C25 MMIO physical overflow")?;
            let virt = paging::map_kernel_mmio(allocator, kernel_cr3, phys, RADEON_C25_PAGE_BYTES)?;
            let p = (virt + in_page) as *mut u32;

            s.persistence_observed = unsafe { core::ptr::read_volatile(p) };
            s.c24_restore_persisted = s.persistence_observed == s.value_expected;
            if !s.c24_restore_persisted {
                return Err("K14.C25 C24-restored SCRATCH_REG0 value did not persist");
            }

            s.pattern_a = derive_pattern_a(s.value_expected);
            s.pattern_b = derive_pattern_b(s.value_expected);
            if s.pattern_a == s.pattern_b || s.pattern_a == s.value_expected
                || s.pattern_b == s.value_expected || s.pattern_a == u32::MAX
                || s.pattern_b == u32::MAX
                || (s.value_expected ^ s.pattern_a).count_ones() != RADEON_C25_PATTERN_BITS_PER_CYCLE
                || (s.value_expected ^ s.pattern_b).count_ones() != RADEON_C25_PATTERN_BITS_PER_CYCLE {
                return Err("K14.C25 runtime pattern derivation violates distinct four-bit contract");
            }

            s.cycle_a_attempted = true;
            let a = run_cycle(p, s.value_expected, s.pattern_a);
            s.cycle_a_pattern_verified = a.pattern_verified;
            s.cycle_a_restore_verified = a.restore_verified;
            s.cycle_a_restore_retry_used = a.restore_retry_used;
            s.cycle_a_pattern_polls = a.pattern_polls;
            s.cycle_a_restore_polls = a.restore_polls;
            s.writes_performed = a.writes;
            if !s.cycle_a_restore_verified {
                return Err("K14.C25 cycle A could not restore original SCRATCH_REG0 value");
            }
            if !s.cycle_a_pattern_verified {
                return Err("K14.C25 cycle A four-bit pattern did not read back exactly");
            }

            s.intercycle_observed = unsafe { core::ptr::read_volatile(p) };
            s.intercycle_restore_persisted = s.intercycle_observed == s.value_expected;
            if !s.intercycle_restore_persisted {
                return Err("K14.C25 cycle A restored value did not persist before cycle B");
            }

            s.cycle_b_attempted = true;
            let b = run_cycle(p, s.value_expected, s.pattern_b);
            s.cycle_b_pattern_verified = b.pattern_verified;
            s.cycle_b_restore_verified = b.restore_verified;
            s.cycle_b_restore_retry_used = b.restore_retry_used;
            s.cycle_b_pattern_polls = b.pattern_polls;
            s.cycle_b_restore_polls = b.restore_polls;
            s.writes_performed = s.writes_performed.saturating_add(b.writes);
            if !s.cycle_b_restore_verified {
                return Err("K14.C25 cycle B could not restore original SCRATCH_REG0 value");
            }
            if !s.cycle_b_pattern_verified {
                return Err("K14.C25 cycle B four-bit pattern did not read back exactly");
            }

            let command_after = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_after_on = command_after & (1 << 1) != 0;
            s.bus_master_after_off = command_after & (1 << 2) == 0;
            if !s.memory_decode_after_on || !s.bus_master_after_off {
                return Err("K14.C25 PCI decode/bus-master safety changed during dual-pattern test");
            }

            s.dual_pattern_verified = s.cycle_a_pattern_verified && s.cycle_a_restore_verified
                && s.intercycle_restore_persisted && s.cycle_b_pattern_verified
                && s.cycle_b_restore_verified;
            s.transaction_fingerprint = fingerprint(&s);
            serial::println(format_args!(
                "[C25HW] GFX12 SCRATCH_REG0 dual multi-bit pattern stability: present=true navi48=true devid={:#06x} target_dwords={:#x} expected={:#010x} persisted={:#010x} patternA={:#010x} patternB={:#010x} intercycle={:#010x} cycleA={} restoreA={} retryA={} cycleB={} restoreB={} retryB={} dual={} changed_bits_each=4 writes={} a_pattern_polls={} a_restore_polls={} b_pattern_polls={} b_restore_polls={} memory_decode_before=true memory_decode_after=true bus_master_before=false bus_master_after=false fingerprint={:#018x} fallback=true",
                s.device_id, s.target_dword_offset, s.value_expected, s.persistence_observed,
                s.pattern_a, s.pattern_b, s.intercycle_observed,
                s.cycle_a_pattern_verified, s.cycle_a_restore_verified,
                s.cycle_a_restore_retry_used, s.cycle_b_pattern_verified,
                s.cycle_b_restore_verified, s.cycle_b_restore_retry_used,
                s.dual_pattern_verified, s.writes_performed,
                s.cycle_a_pattern_polls, s.cycle_a_restore_polls,
                s.cycle_b_pattern_polls, s.cycle_b_restore_polls,
                s.transaction_fingerprint
            ));
        }
    }

    if s.dual_pattern_verified && (!s.transaction_eligible || !s.target_revalidated
        || !s.c24_restore_persisted || !s.intercycle_restore_persisted
        || !s.cycle_a_attempted || !s.cycle_a_pattern_verified || !s.cycle_a_restore_verified
        || !s.cycle_b_attempted || !s.cycle_b_pattern_verified || !s.cycle_b_restore_verified
        || !s.bar5_ready || !s.memory_decode_before_on || !s.memory_decode_after_on
        || !s.bus_master_before_off || !s.bus_master_after_off || !s.c24_pattern_verified
        || !s.c24_restore_verified || !s.c24_target_revalidated) {
        return Err("K14.C25 dual-pattern qualified without every reversible-write safety gate");
    }
    if s.writes_performed > RADEON_C25_MAX_MMIO_WRITES
        || s.arbitrary_mmio_write_enabled || s.mm_index_fallback_used || s.bar_resize_used
        || s.firmware_upload_enabled || s.command_submit_enabled || s.radeon_bus_master_enabled
        || RADEON_C25_ARBITRARY_MMIO_WRITES_ALLOWED || RADEON_C25_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C25_BAR_RESIZE_ALLOWED || RADEON_C25_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C25_COMMAND_SUBMIT_ALLOWED || RADEON_C25_BUS_MASTER_ALLOWED
        || RADEON_C25_CALLER_SUPPLIED_VALUE_ALLOWED || RADEON_C25_CALLER_SUPPLIED_ADDRESS_ALLOWED {
        return Err("K14.C25 capability escaped bounded dual-pattern SCRATCH_REG0 contract");
    }

    serial::println(format_args!(
        "[C25RD] K14.C25 dual-pattern ready: amd_present={} navi48={} profile={} domain={} C19_verified={} C20_ready={} C21_identity={} C22_mutation={} C22_restore={} C23_dual={} C24_pattern={} C24_restore={} C24_target={} revalidated={} C24_persisted={} intercycle={} target_dwords={:#x} BAR5={} eligible={} cycleA={} restoreA={} cycleB={} restoreB={} dual={} writes={} a_pattern_polls={} a_restore_polls={} b_pattern_polls={} b_restore_polls={} memdecode_before={} memdecode_after={} bus_master_before_off={} bus_master_after_off={} c24_fp={:#018x} tx_fp={:#018x} arbitrary=false caller_value=false caller_address=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master=false fallback=true",
        s.amd_present, s.navi48, s.profile_verified, s.exact_domain_live,
        s.c19_snapshot_verified, s.c20_exact_bases_ready, s.c21_identity_verified,
        s.c22_mutation_verified, s.c22_restore_verified, s.c23_dual_cycle_verified,
        s.c24_pattern_verified, s.c24_restore_verified, s.c24_target_revalidated,
        s.target_revalidated, s.c24_restore_persisted, s.intercycle_restore_persisted,
        s.target_dword_offset, s.bar5_ready, s.transaction_eligible,
        s.cycle_a_pattern_verified, s.cycle_a_restore_verified,
        s.cycle_b_pattern_verified, s.cycle_b_restore_verified,
        s.dual_pattern_verified, s.writes_performed,
        s.cycle_a_pattern_polls, s.cycle_a_restore_polls,
        s.cycle_b_pattern_polls, s.cycle_b_restore_polls,
        s.memory_decode_before_on, s.memory_decode_after_on,
        s.bus_master_before_off, s.bus_master_after_off,
        s.c24_transaction_fingerprint, s.transaction_fingerprint
    ));
    *STATE.lock() = s;
    Ok(s)
}

pub fn state() -> C25State { *STATE.lock() }

pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.device_id) << 40)
        | (u64::from(s.revision) << 32)
        | (u64::from(s.writes_performed) << 24);
    for (bit, on) in [
        s.amd_present,                 // bit 0
        s.navi48,                      // bit 1
        s.c24_pattern_verified,        // bit 2
        s.c24_restore_verified,        // bit 3
        s.target_revalidated,          // bit 4
        s.c24_restore_persisted,       // bit 5
        s.transaction_eligible,        // bit 6
        s.cycle_a_attempted,           // bit 7
        s.cycle_a_restore_verified,    // bit 8
        s.cycle_b_attempted,           // bit 9
        s.cycle_b_restore_verified,    // bit 10
        s.bus_master_after_off,        // bit 11
        s.memory_decode_after_on,      // bit 12
        s.dual_pattern_verified,       // bit 13
        s.fallback_armed,              // bit 14
    ].into_iter().enumerate() {
        if on { v |= 1u64 << bit; }
    }
    v
}
