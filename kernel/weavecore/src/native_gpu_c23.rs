//! K14.C23 post-restore persistence and dual-probe stability gate.
//!
//! Frozen C22 proves one bounded reversible one-bit mutation of the exact,
//! checksum-backed GFX12 SCRATCH_REG0 target and exact restoration of the
//! original value. C23 does not widen the writable register set. It first
//! requires the value restored by C22 to persist into this milestone, then
//! executes two distinct internally-derived one-bit probe/restore cycles on
//! the same target. Each cycle requires bounded exact mutation readback and
//! exact restoration; one final restore retry per cycle exists only for
//! recovery of the original scratch value.
//!
//! Caller-selected addresses/values, arbitrary MMIO writes, MM_INDEX/MM_DATA,
//! BAR resizing, firmware upload, command submission and Radeon bus-master
//! enable remain forbidden.

use crate::{
    memory::FrameAllocator,
    native_gpu_binding,
    native_gpu_c19,
    native_gpu_c21,
    native_gpu_c22,
    paging,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C23_ABI_VERSION: u32 = 1;
pub const RADEON_C23_MMIO_BAR_INDEX: u8 = native_gpu_c22::RADEON_C22_MMIO_BAR_INDEX;
pub const RADEON_C23_PAGE_BYTES: u64 = native_gpu_c22::RADEON_C22_PAGE_BYTES;
pub const RADEON_C23_MAX_MUTATION_POLLS_PER_CYCLE: u8 = 32;
pub const RADEON_C23_MAX_RESTORE_POLLS_PER_CYCLE: u8 = 32;
pub const RADEON_C23_MAX_MMIO_WRITES: u8 = 6; // (probe + restore + recovery retry) * 2
pub const RADEON_C23_DUAL_PROBE_ALLOWED: bool = true;
pub const RADEON_C23_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C23_MM_INDEX_FALLBACK_ALLOWED: bool = false;
pub const RADEON_C23_BAR_RESIZE_ALLOWED: bool = false;
pub const RADEON_C23_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C23_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C23_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C23_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false;
pub const RADEON_C23_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct C23State {
    pub amd_present: bool,
    pub navi48: bool,
    pub profile_verified: bool,
    pub exact_domain_live: bool,
    pub c19_snapshot_verified: bool,
    pub c20_exact_bases_ready: bool,
    pub c21_identity_verified: bool,
    pub c22_mutation_verified: bool,
    pub c22_restore_verified: bool,
    pub c22_target_revalidated: bool,
    pub c22_transaction_fingerprint: u64,
    pub target_revalidated: bool,
    pub c22_restore_persisted: bool,
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
    pub probe_a: u32,
    pub probe_b: u32,
    pub cycle_a_attempted: bool,
    pub cycle_a_mutation_verified: bool,
    pub cycle_a_restore_verified: bool,
    pub cycle_a_restore_retry_used: bool,
    pub cycle_b_attempted: bool,
    pub cycle_b_mutation_verified: bool,
    pub cycle_b_restore_verified: bool,
    pub cycle_b_restore_retry_used: bool,
    pub cycle_a_mutation_polls: u8,
    pub cycle_a_restore_polls: u8,
    pub cycle_b_mutation_polls: u8,
    pub cycle_b_restore_polls: u8,
    pub writes_performed: u8,
    pub dual_cycle_verified: bool,
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

impl C23State {
    pub const EMPTY: Self = Self {
        amd_present: false,
        navi48: false,
        profile_verified: false,
        exact_domain_live: false,
        c19_snapshot_verified: false,
        c20_exact_bases_ready: false,
        c21_identity_verified: false,
        c22_mutation_verified: false,
        c22_restore_verified: false,
        c22_target_revalidated: false,
        c22_transaction_fingerprint: 0,
        target_revalidated: false,
        c22_restore_persisted: false,
        intercycle_restore_persisted: false,
        target_dword_offset: 0,
        target_byte_offset: 0,
        bar5_ready: false,
        memory_decode_before_on: false,
        memory_decode_after_on: false,
        bus_master_before_off: false,
        bus_master_after_off: false,
        transaction_eligible: false,
        value_expected: 0,
        persistence_observed: 0,
        intercycle_observed: 0,
        probe_a: 0,
        probe_b: 0,
        cycle_a_attempted: false,
        cycle_a_mutation_verified: false,
        cycle_a_restore_verified: false,
        cycle_a_restore_retry_used: false,
        cycle_b_attempted: false,
        cycle_b_mutation_verified: false,
        cycle_b_restore_verified: false,
        cycle_b_restore_retry_used: false,
        cycle_a_mutation_polls: 0,
        cycle_a_restore_polls: 0,
        cycle_b_mutation_polls: 0,
        cycle_b_restore_polls: 0,
        writes_performed: 0,
        dual_cycle_verified: false,
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

#[derive(Clone, Copy)]
struct CycleResult {
    mutation_verified: bool,
    restore_verified: bool,
    restore_retry_used: bool,
    writes: u8,
    mutation_polls: u8,
    restore_polls: u8,
}

static STATE: SpinLock<C23State> = SpinLock::new(C23State::EMPTY);

/// Derive a second one-bit mutation that differs from C22's first probe.
/// Search high-to-low so it is deterministic, reject all-ones, and never
/// return the first probe. There are always many one-bit alternatives.
pub const fn derive_distinct_one_bit_probe(original: u32, first_probe: u32) -> u32 {
    let mut bit = 32u32;
    while bit > 0 {
        bit -= 1;
        let candidate = original ^ (1u32 << bit);
        if candidate != first_probe && candidate != u32::MAX {
            return candidate;
        }
    }
    original
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

fn poll_exact(p: *mut u32, expected: u32, limit: u8) -> (bool, u8) {
    let mut polls = 0u8;
    while polls < limit {
        let observed = unsafe { core::ptr::read_volatile(p) };
        polls += 1;
        if observed == expected { return (true, polls); }
    }
    (false, polls)
}

fn run_cycle(p: *mut u32, original: u32, probe: u32) -> CycleResult {
    unsafe { core::ptr::write_volatile(p, probe) };
    let (mutation_verified, mutation_polls) = poll_exact(
        p, probe, RADEON_C23_MAX_MUTATION_POLLS_PER_CYCLE
    );

    // Restoration is mandatory even if mutation readback failed.
    unsafe { core::ptr::write_volatile(p, original) };
    let (mut restore_verified, mut restore_polls) = poll_exact(
        p, original, RADEON_C23_MAX_RESTORE_POLLS_PER_CYCLE
    );
    let mut restore_retry_used = false;
    let mut writes = 2u8;

    if !restore_verified {
        restore_retry_used = true;
        unsafe { core::ptr::write_volatile(p, original) };
        writes = 3;
        let (retry_ok, retry_polls) = poll_exact(
            p, original, RADEON_C23_MAX_RESTORE_POLLS_PER_CYCLE
        );
        restore_polls = restore_polls.saturating_add(retry_polls);
        restore_verified = retry_ok;
    }

    CycleResult {
        mutation_verified,
        restore_verified,
        restore_retry_used,
        writes,
        mutation_polls,
        restore_polls,
    }
}

fn fingerprint(s: &C23State) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for byte in s.device_id.to_le_bytes().into_iter()
        .chain([s.revision])
        .chain(s.target_dword_offset.to_le_bytes())
        .chain(s.value_expected.to_le_bytes())
        .chain(s.probe_a.to_le_bytes())
        .chain(s.probe_b.to_le_bytes())
        .chain([s.writes_performed, s.cycle_a_mutation_polls, s.cycle_a_restore_polls,
                s.cycle_b_mutation_polls, s.cycle_b_restore_polls])
        .chain(s.c22_transaction_fingerprint.to_le_bytes()) {
        h ^= u64::from(byte);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn self_test() -> Result<(), &'static str> {
    if K14C23_ABI_VERSION != 1
        || RADEON_C23_MMIO_BAR_INDEX != 5
        || RADEON_C23_PAGE_BYTES != 4096
        || RADEON_C23_MAX_MUTATION_POLLS_PER_CYCLE != 32
        || RADEON_C23_MAX_RESTORE_POLLS_PER_CYCLE != 32
        || RADEON_C23_MAX_MMIO_WRITES != 6
        || !RADEON_C23_DUAL_PROBE_ALLOWED
        || RADEON_C23_ARBITRARY_MMIO_WRITES_ALLOWED
        || RADEON_C23_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C23_BAR_RESIZE_ALLOWED
        || RADEON_C23_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C23_COMMAND_SUBMIT_ALLOWED
        || RADEON_C23_BUS_MASTER_ALLOWED
        || RADEON_C23_CALLER_SUPPLIED_VALUE_ALLOWED
        || RADEON_C23_CALLER_SUPPLIED_ADDRESS_ALLOWED {
        return Err("K14.C23 dual-probe/fail-closed constants invalid");
    }
    for original in [0u32, 1, 2, 3, 0x55aa_55aa, 0x8000_0000, 0x7fff_ffff, 0xffff_fffe] {
        let a = native_gpu_c22::derive_one_bit_probe(original);
        let b = derive_distinct_one_bit_probe(original, a);
        if a == original || b == original || a == b || a == u32::MAX || b == u32::MAX
            || (original ^ a).count_ones() != 1 || (original ^ b).count_ones() != 1 {
            return Err("K14.C23 distinct one-bit probe self-test failed");
        }
    }
    Ok(())
}

pub fn initialize(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<C23State, &'static str> {
    self_test()?;
    let c19 = native_gpu_c19::state();
    let c21 = native_gpu_c21::state();
    let c22 = native_gpu_c22::state();
    let mut s = C23State {
        amd_present: c22.amd_present,
        navi48: c22.navi48,
        profile_verified: c22.profile_verified,
        exact_domain_live: c22.exact_domain_live,
        c19_snapshot_verified: c22.c19_snapshot_verified,
        c20_exact_bases_ready: c22.c20_exact_bases_ready,
        c21_identity_verified: c22.c21_identity_verified,
        c22_mutation_verified: c22.mutation_verified,
        c22_restore_verified: c22.restore_verified,
        c22_target_revalidated: c22.target_revalidated,
        c22_transaction_fingerprint: c22.transaction_fingerprint,
        target_dword_offset: c22.target_dword_offset,
        target_byte_offset: c22.target_byte_offset,
        value_expected: c22.value_before,
        snapshot_fingerprint: c22.snapshot_fingerprint,
        device_id: c22.device_id,
        revision: c22.revision,
        ..C23State::EMPTY
    };

    serial::println(format_args!(
        "[C23PS] post-restore persistence gate: source=C22_exact_restore same_target=true require_initial_value_match=true require_intercycle_value_match=true"
    ));
    serial::println(format_args!(
        "[C23PG] dual-probe policy: require=C22_mutation_verified+C22_restore_verified+same_checksum_snapshot+same_exact_target+BAR5+memory_decode_on+bus_master_off; mutation_polls_per_cycle<={} restore_polls_per_cycle<={} max_writes={} arbitrary=false caller_value=false caller_address=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master_enable=false",
        RADEON_C23_MAX_MUTATION_POLLS_PER_CYCLE,
        RADEON_C23_MAX_RESTORE_POLLS_PER_CYCLE,
        RADEON_C23_MAX_MMIO_WRITES
    ));
    serial::println(format_args!(
        "[C23TX] stability transaction contract: verify_C22_restored_value -> probe_A -> exact_A -> restore -> exact_restore_A -> verify_intercycle_persistence -> distinct_probe_B -> exact_B -> restore -> exact_restore_B -> recheck_memory_decode_and_bus_master"
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C23HW] GFX12 SCRATCH_REG0 dual-probe stability: present=false qemu_deferred=true persistence=false cycleA=false cycleB=false fallback=true"
        ));
    } else {
        let c22_ready = s.navi48 && s.profile_verified && s.exact_domain_live
            && s.c19_snapshot_verified && s.c20_exact_bases_ready
            && s.c21_identity_verified && s.c22_mutation_verified
            && s.c22_restore_verified && s.c22_target_revalidated
            && s.c22_transaction_fingerprint != 0 && s.snapshot_fingerprint != 0
            && s.snapshot_fingerprint == c19.snapshot_fingerprint;
        if !c22_ready {
            serial::println(format_args!(
                "[C23HW] GFX12 SCRATCH_REG0 dual-probe stability: present=true devid={:#06x} C22_mutation={} C22_restore={} C22_target={} snapshot_match={} attempted=false reason=C22_stability_gate_not_ready fallback=true",
                s.device_id, s.c22_mutation_verified, s.c22_restore_verified,
                s.c22_target_revalidated,
                s.snapshot_fingerprint != 0 && s.snapshot_fingerprint == c19.snapshot_fingerprint
            ));
        } else {
            let target = native_gpu_c19::with_verified_snapshot(native_gpu_c21::resolve_gfx12_scratch_reg0)
                .ok_or("K14.C23 C19 verified snapshot unavailable")??;
            s.target_revalidated = target.valid
                && target.target_dword_offset == s.target_dword_offset
                && target.target_byte_offset == s.target_byte_offset
                && target.gc_segment1_base_dwords == c21.gc_segment1_base_dwords;
            if !s.target_revalidated {
                return Err("K14.C23 live target no longer matches frozen C22 proof");
            }

            let function = selected_function();
            let command_before = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_before_on = command_before & (1 << 1) != 0;
            s.bus_master_before_off = command_before & (1 << 2) == 0;
            if !s.memory_decode_before_on {
                return Err("K14.C23 Radeon PCI memory decoding is disabled");
            }
            if !s.bus_master_before_off {
                return Err("K14.C23 Radeon bus mastering unexpectedly enabled before dual-probe stability test");
            }

            let bar = pci::memory_bar_base(function, RADEON_C23_MMIO_BAR_INDEX)
                .ok_or("K14.C23 Radeon BAR5 unavailable")?;
            s.bar5_ready = true;
            s.transaction_eligible = true;

            let page_off = s.target_byte_offset & !(RADEON_C23_PAGE_BYTES - 1);
            let in_page = s.target_byte_offset & (RADEON_C23_PAGE_BYTES - 1);
            if in_page + 4 > RADEON_C23_PAGE_BYTES {
                return Err("K14.C23 reviewed target crosses MMIO page boundary");
            }
            let phys = bar.checked_add(page_off).ok_or("K14.C23 MMIO physical overflow")?;
            let virt = paging::map_kernel_mmio(allocator, kernel_cr3, phys, RADEON_C23_PAGE_BYTES)?;
            let p = (virt + in_page) as *mut u32;

            s.persistence_observed = unsafe { core::ptr::read_volatile(p) };
            s.c22_restore_persisted = s.persistence_observed == s.value_expected;
            if !s.c22_restore_persisted {
                return Err("K14.C23 C22-restored SCRATCH_REG0 value did not persist");
            }

            s.probe_a = native_gpu_c22::derive_one_bit_probe(s.value_expected);
            s.probe_b = derive_distinct_one_bit_probe(s.value_expected, s.probe_a);
            if s.probe_a == s.probe_b || s.probe_a == s.value_expected || s.probe_b == s.value_expected
                || s.probe_a == u32::MAX || s.probe_b == u32::MAX
                || (s.value_expected ^ s.probe_a).count_ones() != 1
                || (s.value_expected ^ s.probe_b).count_ones() != 1 {
                return Err("K14.C23 runtime probe derivation violates distinct one-bit contract");
            }

            s.cycle_a_attempted = true;
            let a = run_cycle(p, s.value_expected, s.probe_a);
            s.cycle_a_mutation_verified = a.mutation_verified;
            s.cycle_a_restore_verified = a.restore_verified;
            s.cycle_a_restore_retry_used = a.restore_retry_used;
            s.cycle_a_mutation_polls = a.mutation_polls;
            s.cycle_a_restore_polls = a.restore_polls;
            s.writes_performed = a.writes;
            if !s.cycle_a_restore_verified {
                return Err("K14.C23 cycle A could not restore original SCRATCH_REG0 value");
            }
            if !s.cycle_a_mutation_verified {
                return Err("K14.C23 cycle A one-bit mutation did not read back exactly");
            }

            s.intercycle_observed = unsafe { core::ptr::read_volatile(p) };
            s.intercycle_restore_persisted = s.intercycle_observed == s.value_expected;
            if !s.intercycle_restore_persisted {
                return Err("K14.C23 cycle A restored value did not persist before cycle B");
            }

            s.cycle_b_attempted = true;
            let b = run_cycle(p, s.value_expected, s.probe_b);
            s.cycle_b_mutation_verified = b.mutation_verified;
            s.cycle_b_restore_verified = b.restore_verified;
            s.cycle_b_restore_retry_used = b.restore_retry_used;
            s.cycle_b_mutation_polls = b.mutation_polls;
            s.cycle_b_restore_polls = b.restore_polls;
            s.writes_performed = s.writes_performed.saturating_add(b.writes);
            if !s.cycle_b_restore_verified {
                return Err("K14.C23 cycle B could not restore original SCRATCH_REG0 value");
            }
            if !s.cycle_b_mutation_verified {
                return Err("K14.C23 cycle B one-bit mutation did not read back exactly");
            }

            let command_after = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_after_on = command_after & (1 << 1) != 0;
            s.bus_master_after_off = command_after & (1 << 2) == 0;
            if !s.memory_decode_after_on || !s.bus_master_after_off {
                return Err("K14.C23 PCI decode/bus-master safety changed during dual-probe stability test");
            }

            s.dual_cycle_verified = s.c22_restore_persisted
                && s.intercycle_restore_persisted
                && s.cycle_a_mutation_verified && s.cycle_a_restore_verified
                && s.cycle_b_mutation_verified && s.cycle_b_restore_verified;
            s.transaction_fingerprint = fingerprint(&s);
            serial::println(format_args!(
                "[C23HW] GFX12 SCRATCH_REG0 dual-probe stability: present=true navi48=true devid={:#06x} target_dwords={:#x} expected={:#010x} persisted={:#010x} probeA={:#010x} A_mutation={} A_restore={} A_retry={} intercycle={:#010x} probeB={:#010x} B_mutation={} B_restore={} B_retry={} writes={} memory_decode_before=true memory_decode_after=true bus_master_before=false bus_master_after=false fingerprint={:#018x} fallback=true",
                s.device_id, s.target_dword_offset, s.value_expected, s.persistence_observed,
                s.probe_a, s.cycle_a_mutation_verified, s.cycle_a_restore_verified,
                s.cycle_a_restore_retry_used, s.intercycle_observed, s.probe_b,
                s.cycle_b_mutation_verified, s.cycle_b_restore_verified,
                s.cycle_b_restore_retry_used, s.writes_performed, s.transaction_fingerprint
            ));
        }
    }

    if s.dual_cycle_verified && (!s.transaction_eligible || !s.target_revalidated
        || !s.c22_restore_persisted || !s.intercycle_restore_persisted
        || !s.cycle_a_attempted || !s.cycle_b_attempted
        || !s.bar5_ready || !s.memory_decode_before_on || !s.memory_decode_after_on
        || !s.bus_master_before_off || !s.bus_master_after_off
        || !s.c22_mutation_verified || !s.c22_restore_verified) {
        return Err("K14.C23 dual-probe qualified without every persistence/safety gate");
    }
    if s.writes_performed > RADEON_C23_MAX_MMIO_WRITES
        || s.arbitrary_mmio_write_enabled || s.mm_index_fallback_used || s.bar_resize_used
        || s.firmware_upload_enabled || s.command_submit_enabled || s.radeon_bus_master_enabled
        || RADEON_C23_ARBITRARY_MMIO_WRITES_ALLOWED || RADEON_C23_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C23_BAR_RESIZE_ALLOWED || RADEON_C23_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C23_COMMAND_SUBMIT_ALLOWED || RADEON_C23_BUS_MASTER_ALLOWED
        || RADEON_C23_CALLER_SUPPLIED_VALUE_ALLOWED || RADEON_C23_CALLER_SUPPLIED_ADDRESS_ALLOWED {
        return Err("K14.C23 capability escaped dual-probe SCRATCH_REG0 contract");
    }

    serial::println(format_args!(
        "[C23RD] K14.C23 stability ready: amd_present={} navi48={} profile={} domain={} C19_verified={} C20_ready={} C21_identity={} C22_mutation={} C22_restore={} C22_target={} revalidated={} C22_persisted={} intercycle_persisted={} target_dwords={:#x} BAR5={} eligible={} cycleA_mutation={} cycleA_restore={} cycleB_mutation={} cycleB_restore={} writes={} memdecode_before={} memdecode_after={} bus_master_before_off={} bus_master_after_off={} c22_fp={:#018x} tx_fp={:#018x} arbitrary=false caller_value=false caller_address=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master=false fallback=true",
        s.amd_present, s.navi48, s.profile_verified, s.exact_domain_live,
        s.c19_snapshot_verified, s.c20_exact_bases_ready, s.c21_identity_verified,
        s.c22_mutation_verified, s.c22_restore_verified, s.c22_target_revalidated,
        s.target_revalidated, s.c22_restore_persisted, s.intercycle_restore_persisted,
        s.target_dword_offset, s.bar5_ready, s.transaction_eligible,
        s.cycle_a_mutation_verified, s.cycle_a_restore_verified,
        s.cycle_b_mutation_verified, s.cycle_b_restore_verified, s.writes_performed,
        s.memory_decode_before_on, s.memory_decode_after_on,
        s.bus_master_before_off, s.bus_master_after_off,
        s.c22_transaction_fingerprint, s.transaction_fingerprint
    ));
    *STATE.lock() = s;
    Ok(s)
}

pub fn state() -> C23State { *STATE.lock() }

pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.device_id) << 40)
        | (u64::from(s.revision) << 32)
        | (u64::from(s.writes_performed) << 24);
    for (bit, on) in [
        s.amd_present,                  // bit 0
        s.navi48,                       // bit 1
        s.c22_mutation_verified,        // bit 2
        s.c22_restore_verified,         // bit 3
        s.target_revalidated,           // bit 4
        s.c22_restore_persisted,        // bit 5
        s.intercycle_restore_persisted, // bit 6
        s.cycle_a_mutation_verified,    // bit 7
        s.cycle_a_restore_verified,     // bit 8
        s.cycle_b_mutation_verified,    // bit 9
        s.cycle_b_restore_verified,     // bit 10
        s.bus_master_before_off,        // bit 11
        s.bus_master_after_off,         // bit 12
        s.dual_cycle_verified,          // bit 13
        s.fallback_armed,                // bit 14
    ].into_iter().enumerate() {
        if on { v |= 1u64 << bit; }
    }
    v
}
