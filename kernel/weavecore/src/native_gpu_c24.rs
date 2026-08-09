//! K14.C24 bounded reversible multi-bit GFX12 SCRATCH_REG0 pattern gate.
//!
//! Frozen C23 proves that the exact checksum-backed GFX12 SCRATCH_REG0 target
//! retains the C22-restored value and survives two distinct one-bit probe /
//! restore cycles. C24 does not widen the writable register set. It increases
//! only the data-pattern depth by applying one deterministic four-bit XOR
//! pattern to that same target, requiring bounded exact readback, then restoring
//! the original value exactly.
//!
//! The pattern is derived internally. Caller-selected addresses/values,
//! arbitrary MMIO writes, MM_INDEX/MM_DATA, BAR resizing, firmware upload,
//! command submission, and Radeon bus-master enable remain forbidden.

use crate::{
    memory::FrameAllocator,
    native_gpu_binding,
    native_gpu_c19,
    native_gpu_c21,
    native_gpu_c23,
    paging,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C24_ABI_VERSION: u32 = 1;
pub const RADEON_C24_MMIO_BAR_INDEX: u8 = native_gpu_c23::RADEON_C23_MMIO_BAR_INDEX;
pub const RADEON_C24_PAGE_BYTES: u64 = native_gpu_c23::RADEON_C23_PAGE_BYTES;
pub const RADEON_C24_MAX_PATTERN_POLLS: u8 = 32;
pub const RADEON_C24_MAX_RESTORE_POLLS: u8 = 32;
pub const RADEON_C24_MAX_MMIO_WRITES: u8 = 3; // pattern + mandatory restore + one recovery restore
pub const RADEON_C24_PATTERN_BITS: u32 = 4;
pub const RADEON_C24_MULTI_BIT_PATTERN_ALLOWED: bool = true;
pub const RADEON_C24_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C24_MM_INDEX_FALLBACK_ALLOWED: bool = false;
pub const RADEON_C24_BAR_RESIZE_ALLOWED: bool = false;
pub const RADEON_C24_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C24_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C24_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C24_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false;
pub const RADEON_C24_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct C24State {
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
    pub c23_target_revalidated: bool,
    pub c23_transaction_fingerprint: u64,
    pub target_revalidated: bool,
    pub c23_restore_persisted: bool,
    pub target_dword_offset: u64,
    pub target_byte_offset: u64,
    pub bar5_ready: bool,
    pub memory_decode_before_on: bool,
    pub memory_decode_after_on: bool,
    pub bus_master_before_off: bool,
    pub bus_master_after_off: bool,
    pub transaction_eligible: bool,
    pub pattern_attempted: bool,
    pub pattern_verified: bool,
    pub restore_attempted: bool,
    pub restore_verified: bool,
    pub restore_retry_used: bool,
    pub writes_performed: u8,
    pub pattern_polls: u8,
    pub restore_polls: u8,
    pub value_expected: u32,
    pub persistence_observed: u32,
    pub pattern_value: u32,
    pub pattern_observed: u32,
    pub restored_value: u32,
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

impl C24State {
    pub const EMPTY: Self = Self {
        amd_present: false, navi48: false, profile_verified: false,
        exact_domain_live: false, c19_snapshot_verified: false,
        c20_exact_bases_ready: false, c21_identity_verified: false,
        c22_mutation_verified: false, c22_restore_verified: false,
        c23_dual_cycle_verified: false, c23_target_revalidated: false,
        c23_transaction_fingerprint: 0, target_revalidated: false,
        c23_restore_persisted: false, target_dword_offset: 0,
        target_byte_offset: 0, bar5_ready: false, memory_decode_before_on: false,
        memory_decode_after_on: false, bus_master_before_off: false,
        bus_master_after_off: false, transaction_eligible: false,
        pattern_attempted: false, pattern_verified: false,
        restore_attempted: false, restore_verified: false,
        restore_retry_used: false, writes_performed: 0, pattern_polls: 0,
        restore_polls: 0, value_expected: 0, persistence_observed: 0,
        pattern_value: 0, pattern_observed: 0, restored_value: 0,
        snapshot_fingerprint: 0, transaction_fingerprint: 0,
        arbitrary_mmio_write_enabled: false, mm_index_fallback_used: false,
        bar_resize_used: false, firmware_upload_enabled: false,
        command_submit_enabled: false, radeon_bus_master_enabled: false,
        fallback_armed: true, device_id: 0, revision: 0,
    };
}

static STATE: SpinLock<C24State> = SpinLock::new(C24State::EMPTY);

/// Derive C24's only permitted mutation value. Normally toggle bits 0..3.
/// If that exact pattern would produce all ones, toggle bits 4..7 instead.
/// Either result differs from the original by exactly four bits.
pub const fn derive_four_bit_pattern(original: u32) -> u32 {
    let low = original ^ 0x0000_000f;
    if low != u32::MAX { low } else { original ^ 0x0000_00f0 }
}

fn selected_function() -> pci::PciFunction {
    let b = native_gpu_binding::state();
    pci::PciFunction {
        bus: b.selected_bus, device: b.selected_device, function: b.selected_function,
        vendor_id: 0, device_id: 0, class_code: 0, subclass: 0,
        programming_interface: 0, revision: 0, header_type: 0,
    }
}

fn poll_exact(p: *mut u32, expected: u32, polls: &mut u8, limit: u8) -> (bool, u32) {
    let mut observed = 0u32;
    while *polls < limit {
        observed = unsafe { core::ptr::read_volatile(p) };
        *polls += 1;
        if observed == expected { return (true, observed); }
    }
    (false, observed)
}

fn fingerprint(s: &C24State) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for byte in s.device_id.to_le_bytes().into_iter()
        .chain([s.revision])
        .chain(s.target_dword_offset.to_le_bytes())
        .chain(s.value_expected.to_le_bytes())
        .chain(s.pattern_value.to_le_bytes())
        .chain(s.pattern_observed.to_le_bytes())
        .chain(s.restored_value.to_le_bytes())
        .chain([s.writes_performed, s.pattern_polls, s.restore_polls])
        .chain(s.c23_transaction_fingerprint.to_le_bytes()) {
        h ^= u64::from(byte);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn self_test() -> Result<(), &'static str> {
    if K14C24_ABI_VERSION != 1
        || RADEON_C24_MMIO_BAR_INDEX != 5
        || RADEON_C24_PAGE_BYTES != 4096
        || RADEON_C24_MAX_PATTERN_POLLS != 32
        || RADEON_C24_MAX_RESTORE_POLLS != 32
        || RADEON_C24_MAX_MMIO_WRITES != 3
        || RADEON_C24_PATTERN_BITS != 4
        || !RADEON_C24_MULTI_BIT_PATTERN_ALLOWED
        || RADEON_C24_ARBITRARY_MMIO_WRITES_ALLOWED
        || RADEON_C24_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C24_BAR_RESIZE_ALLOWED
        || RADEON_C24_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C24_COMMAND_SUBMIT_ALLOWED
        || RADEON_C24_BUS_MASTER_ALLOWED
        || RADEON_C24_CALLER_SUPPLIED_VALUE_ALLOWED
        || RADEON_C24_CALLER_SUPPLIED_ADDRESS_ALLOWED {
        return Err("K14.C24 multi-bit-pattern/fail-closed constants invalid");
    }
    for original in [0u32, 1, 0x0f, 0xf0, 0x55aa_55aa, 0xffff_fff0,
                     0xffff_ff0f, 0x8000_0000, 0xffff_fffe] {
        let pattern = derive_four_bit_pattern(original);
        if pattern == original || pattern == u32::MAX
            || (original ^ pattern).count_ones() != RADEON_C24_PATTERN_BITS {
            return Err("K14.C24 deterministic four-bit pattern self-test failed");
        }
    }
    Ok(())
}

pub fn initialize(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<C24State, &'static str> {
    self_test()?;
    let c19 = native_gpu_c19::state();
    let c21 = native_gpu_c21::state();
    let c23 = native_gpu_c23::state();
    let mut s = C24State {
        amd_present: c23.amd_present,
        navi48: c23.navi48,
        profile_verified: c23.profile_verified,
        exact_domain_live: c23.exact_domain_live,
        c19_snapshot_verified: c23.c19_snapshot_verified,
        c20_exact_bases_ready: c23.c20_exact_bases_ready,
        c21_identity_verified: c23.c21_identity_verified,
        c22_mutation_verified: c23.c22_mutation_verified,
        c22_restore_verified: c23.c22_restore_verified,
        c23_dual_cycle_verified: c23.dual_cycle_verified,
        c23_target_revalidated: c23.target_revalidated,
        c23_transaction_fingerprint: c23.transaction_fingerprint,
        target_dword_offset: c23.target_dword_offset,
        target_byte_offset: c23.target_byte_offset,
        value_expected: c23.value_expected,
        snapshot_fingerprint: c23.snapshot_fingerprint,
        device_id: c23.device_id,
        revision: c23.revision,
        ..C24State::EMPTY
    };

    serial::println(format_args!(
        "[C24PT] reversible multi-bit pattern: source=C23_exact_restore derivation=XOR_0x0000000f_or_0x000000f0 changed_bits=4 caller_value=false caller_address=false"
    ));
    serial::println(format_args!(
        "[C24PG] multi-bit-write policy: require=C23_dual_cycle_verified+C23_target_revalidated+same_checksum_snapshot+same_exact_target+BAR5+memory_decode_on+bus_master_off; pattern_polls<={} restore_polls<={} max_writes={} arbitrary=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master_enable=false",
        RADEON_C24_MAX_PATTERN_POLLS, RADEON_C24_MAX_RESTORE_POLLS,
        RADEON_C24_MAX_MMIO_WRITES
    ));
    serial::println(format_args!(
        "[C24TX] reversible pattern contract: verify_C23_restored_value -> derive_internal_4bit_pattern -> write_pattern -> exact_pattern_readback -> mandatory_original_restore -> exact_restore -> recheck_memory_decode_and_bus_master"
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C24HW] GFX12 SCRATCH_REG0 reversible multi-bit pattern: present=false qemu_deferred=true persisted=false attempted=false pattern=false restore=false fallback=true"
        ));
    } else {
        let c23_ready = s.navi48 && s.profile_verified && s.exact_domain_live
            && s.c19_snapshot_verified && s.c20_exact_bases_ready
            && s.c21_identity_verified && s.c22_mutation_verified
            && s.c22_restore_verified && s.c23_dual_cycle_verified
            && s.c23_target_revalidated && s.c23_transaction_fingerprint != 0
            && s.snapshot_fingerprint != 0
            && s.snapshot_fingerprint == c19.snapshot_fingerprint;
        if !c23_ready {
            serial::println(format_args!(
                "[C24HW] GFX12 SCRATCH_REG0 reversible multi-bit pattern: present=true devid={:#06x} C23_dual={} C23_target={} snapshot_match={} attempted=false reason=C23_pattern_gate_not_ready fallback=true",
                s.device_id, s.c23_dual_cycle_verified, s.c23_target_revalidated,
                s.snapshot_fingerprint != 0 && s.snapshot_fingerprint == c19.snapshot_fingerprint
            ));
        } else {
            let target = native_gpu_c19::with_verified_snapshot(native_gpu_c21::resolve_gfx12_scratch_reg0)
                .ok_or("K14.C24 C19 verified snapshot unavailable")??;
            s.target_revalidated = target.valid
                && target.target_dword_offset == s.target_dword_offset
                && target.target_byte_offset == s.target_byte_offset
                && target.gc_segment1_base_dwords == c21.gc_segment1_base_dwords;
            if !s.target_revalidated {
                return Err("K14.C24 live target no longer matches frozen C23 proof");
            }

            let function = selected_function();
            let command_before = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_before_on = command_before & (1 << 1) != 0;
            s.bus_master_before_off = command_before & (1 << 2) == 0;
            if !s.memory_decode_before_on {
                return Err("K14.C24 Radeon PCI memory decoding is disabled");
            }
            if !s.bus_master_before_off {
                return Err("K14.C24 Radeon bus mastering unexpectedly enabled before multi-bit pattern test");
            }

            let bar = pci::memory_bar_base(function, RADEON_C24_MMIO_BAR_INDEX)
                .ok_or("K14.C24 Radeon BAR5 unavailable")?;
            s.bar5_ready = true;
            s.transaction_eligible = true;

            let page_off = s.target_byte_offset & !(RADEON_C24_PAGE_BYTES - 1);
            let in_page = s.target_byte_offset & (RADEON_C24_PAGE_BYTES - 1);
            if in_page + 4 > RADEON_C24_PAGE_BYTES {
                return Err("K14.C24 reviewed target crosses MMIO page boundary");
            }
            let phys = bar.checked_add(page_off).ok_or("K14.C24 MMIO physical overflow")?;
            let virt = paging::map_kernel_mmio(allocator, kernel_cr3, phys, RADEON_C24_PAGE_BYTES)?;
            let p = (virt + in_page) as *mut u32;

            s.persistence_observed = unsafe { core::ptr::read_volatile(p) };
            s.c23_restore_persisted = s.persistence_observed == s.value_expected;
            if !s.c23_restore_persisted {
                return Err("K14.C24 C23-restored SCRATCH_REG0 value did not persist");
            }

            s.pattern_value = derive_four_bit_pattern(s.value_expected);
            if s.pattern_value == s.value_expected || s.pattern_value == u32::MAX
                || (s.value_expected ^ s.pattern_value).count_ones() != RADEON_C24_PATTERN_BITS {
                return Err("K14.C24 derived pattern violates four-bit mutation contract");
            }

            s.pattern_attempted = true;
            unsafe { core::ptr::write_volatile(p, s.pattern_value) };
            s.writes_performed = 1;
            let (pattern_ok, pattern_observed) = poll_exact(
                p, s.pattern_value, &mut s.pattern_polls, RADEON_C24_MAX_PATTERN_POLLS
            );
            s.pattern_observed = pattern_observed;
            s.pattern_verified = pattern_ok;

            // Restore is mandatory regardless of pattern readback result.
            s.restore_attempted = true;
            unsafe { core::ptr::write_volatile(p, s.value_expected) };
            s.writes_performed = 2;
            let (restore_ok, restored) = poll_exact(
                p, s.value_expected, &mut s.restore_polls, RADEON_C24_MAX_RESTORE_POLLS
            );
            s.restored_value = restored;
            s.restore_verified = restore_ok;

            if !s.restore_verified {
                s.restore_retry_used = true;
                unsafe { core::ptr::write_volatile(p, s.value_expected) };
                s.writes_performed = 3;
                let mut retry_polls = 0u8;
                let (retry_ok, retry_value) = poll_exact(
                    p, s.value_expected, &mut retry_polls, RADEON_C24_MAX_RESTORE_POLLS
                );
                s.restore_polls = s.restore_polls.saturating_add(retry_polls);
                s.restored_value = retry_value;
                s.restore_verified = retry_ok;
            }

            let command_after = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_after_on = command_after & (1 << 1) != 0;
            s.bus_master_after_off = command_after & (1 << 2) == 0;
            if !s.memory_decode_after_on || !s.bus_master_after_off {
                return Err("K14.C24 PCI decode/bus-master safety changed during multi-bit pattern test");
            }
            if !s.restore_verified {
                return Err("K14.C24 could not verify restoration of original SCRATCH_REG0 value");
            }
            if !s.pattern_verified {
                return Err("K14.C24 four-bit SCRATCH_REG0 pattern did not read back exactly");
            }

            s.transaction_fingerprint = fingerprint(&s);
            serial::println(format_args!(
                "[C24HW] GFX12 SCRATCH_REG0 reversible multi-bit pattern: present=true navi48=true devid={:#06x} target_dwords={:#x} expected={:#010x} persisted={:#010x} pattern={:#010x} observed={:#010x} restored={:#010x} changed_bits=4 writes={} pattern_polls={} restore_polls={} retry={} memory_decode_before=true memory_decode_after=true bus_master_before=false bus_master_after=false fingerprint={:#018x} fallback=true",
                s.device_id, s.target_dword_offset, s.value_expected, s.persistence_observed,
                s.pattern_value, s.pattern_observed, s.restored_value, s.writes_performed,
                s.pattern_polls, s.restore_polls, s.restore_retry_used,
                s.transaction_fingerprint
            ));
        }
    }

    if s.pattern_verified && (!s.transaction_eligible || !s.pattern_attempted
        || !s.restore_attempted || !s.restore_verified || !s.target_revalidated
        || !s.c23_restore_persisted || !s.bar5_ready || !s.memory_decode_before_on
        || !s.memory_decode_after_on || !s.bus_master_before_off || !s.bus_master_after_off
        || !s.c23_dual_cycle_verified) {
        return Err("K14.C24 pattern qualified without every reversible-write safety gate");
    }
    if s.writes_performed > RADEON_C24_MAX_MMIO_WRITES
        || s.arbitrary_mmio_write_enabled || s.mm_index_fallback_used || s.bar_resize_used
        || s.firmware_upload_enabled || s.command_submit_enabled || s.radeon_bus_master_enabled
        || RADEON_C24_ARBITRARY_MMIO_WRITES_ALLOWED || RADEON_C24_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C24_BAR_RESIZE_ALLOWED || RADEON_C24_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C24_COMMAND_SUBMIT_ALLOWED || RADEON_C24_BUS_MASTER_ALLOWED
        || RADEON_C24_CALLER_SUPPLIED_VALUE_ALLOWED || RADEON_C24_CALLER_SUPPLIED_ADDRESS_ALLOWED {
        return Err("K14.C24 capability escaped bounded reversible multi-bit SCRATCH_REG0 contract");
    }

    serial::println(format_args!(
        "[C24RD] K14.C24 multi-bit-pattern ready: amd_present={} navi48={} profile={} domain={} C19_verified={} C20_ready={} C21_identity={} C22_mutation={} C22_restore={} C23_dual={} C23_target={} revalidated={} C23_persisted={} target_dwords={:#x} BAR5={} eligible={} attempted={} pattern={} restored={} retry={} writes={} pattern_polls={} restore_polls={} memdecode_before={} memdecode_after={} bus_master_before_off={} bus_master_after_off={} c23_fp={:#018x} tx_fp={:#018x} arbitrary=false caller_value=false caller_address=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master=false fallback=true",
        s.amd_present, s.navi48, s.profile_verified, s.exact_domain_live,
        s.c19_snapshot_verified, s.c20_exact_bases_ready, s.c21_identity_verified,
        s.c22_mutation_verified, s.c22_restore_verified, s.c23_dual_cycle_verified,
        s.c23_target_revalidated, s.target_revalidated, s.c23_restore_persisted,
        s.target_dword_offset, s.bar5_ready, s.transaction_eligible,
        s.pattern_attempted, s.pattern_verified, s.restore_verified,
        s.restore_retry_used, s.writes_performed, s.pattern_polls, s.restore_polls,
        s.memory_decode_before_on, s.memory_decode_after_on,
        s.bus_master_before_off, s.bus_master_after_off,
        s.c23_transaction_fingerprint, s.transaction_fingerprint
    ));
    *STATE.lock() = s;
    Ok(s)
}

pub fn state() -> C24State { *STATE.lock() }

pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.device_id) << 40)
        | (u64::from(s.revision) << 32)
        | (u64::from(s.writes_performed) << 24);
    for (bit, on) in [
        s.amd_present,               // bit 0
        s.navi48,                    // bit 1
        s.c23_dual_cycle_verified,   // bit 2
        s.c23_target_revalidated,    // bit 3
        s.target_revalidated,        // bit 4
        s.c23_restore_persisted,     // bit 5
        s.transaction_eligible,      // bit 6
        s.pattern_attempted,         // bit 7
        s.restore_attempted,         // bit 8
        s.restore_verified,          // bit 9
        s.bus_master_before_off,     // bit 10
        s.bus_master_after_off,      // bit 11
        s.memory_decode_after_on,    // bit 12
        s.pattern_verified,          // bit 13
        s.fallback_armed,            // bit 14
    ].into_iter().enumerate() {
        if on { v |= 1u64 << bit; }
    }
    v
}
