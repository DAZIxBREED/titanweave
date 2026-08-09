//! K14.C22 bounded reversible GFX12 SCRATCH_REG0 mutation gate.
//!
//! Frozen C21 proves the exact checksum-backed GFX12 SCRATCH_REG0 target and,
//! on supported bare metal, completes a bus-master-off identity MMIO write.
//! C22 advances exactly one step: it permits a deterministic one-bit mutation
//! of that same source-reviewed scratch register, requires bounded readback of
//! the changed value, and then requires restoration of the original value.
//!
//! The probe value is not caller supplied. For a nonzero original value C22
//! clears exactly the least-significant set bit (`value & (value - 1)`). For
//! zero it writes one. Therefore the probe is guaranteed to differ from the
//! original while remaining a single-bit transition and never becoming all
//! ones. No other address or arbitrary data value is writable.
//!
//! C22 still forbids MM_INDEX/MM_DATA fallback, BAR resizing, firmware upload,
//! command submission, and Radeon bus-master enable. If probe verification or
//! restoration fails, qualification fails closed. A final bounded restore
//! retry is allowed only to maximize recovery of the original scratch value.

use crate::{
    memory::FrameAllocator,
    native_gpu_binding,
    native_gpu_c19,
    native_gpu_c21,
    paging,
    pci,
    serial,
    sync::SpinLock,
};

pub const K14C22_ABI_VERSION: u32 = 1;
pub const RADEON_C22_MMIO_BAR_INDEX: u8 = native_gpu_c21::RADEON_C21_MMIO_BAR_INDEX;
pub const RADEON_C22_PAGE_BYTES: u64 = native_gpu_c21::RADEON_C21_PAGE_BYTES;
pub const RADEON_C22_MAX_MUTATION_POLLS: u8 = 32;
pub const RADEON_C22_MAX_RESTORE_POLLS: u8 = 32;
pub const RADEON_C22_MAX_MMIO_WRITES: u8 = 3; // probe + restore + one final restore retry
pub const RADEON_C22_ONE_BIT_SCRATCH_MUTATION_ALLOWED: bool = true;
pub const RADEON_C22_ARBITRARY_MMIO_WRITES_ALLOWED: bool = false;
pub const RADEON_C22_MM_INDEX_FALLBACK_ALLOWED: bool = false;
pub const RADEON_C22_BAR_RESIZE_ALLOWED: bool = false;
pub const RADEON_C22_FIRMWARE_UPLOAD_ALLOWED: bool = false;
pub const RADEON_C22_COMMAND_SUBMIT_ALLOWED: bool = false;
pub const RADEON_C22_BUS_MASTER_ALLOWED: bool = false;
pub const RADEON_C22_CALLER_SUPPLIED_VALUE_ALLOWED: bool = false;
pub const RADEON_C22_CALLER_SUPPLIED_ADDRESS_ALLOWED: bool = false;

#[derive(Clone, Copy, Debug)]
pub struct C22State {
    pub amd_present: bool,
    pub navi48: bool,
    pub profile_verified: bool,
    pub exact_domain_live: bool,
    pub c19_snapshot_verified: bool,
    pub c20_exact_bases_ready: bool,
    pub c21_identity_verified: bool,
    pub c21_target_reused: bool,
    pub target_revalidated: bool,
    pub target_dword_offset: u64,
    pub target_byte_offset: u64,
    pub bar5_ready: bool,
    pub memory_decode_before_on: bool,
    pub memory_decode_after_on: bool,
    pub bus_master_before_off: bool,
    pub bus_master_after_off: bool,
    pub transaction_eligible: bool,
    pub mutation_attempted: bool,
    pub mutation_verified: bool,
    pub restore_attempted: bool,
    pub restore_verified: bool,
    pub restore_retry_used: bool,
    pub writes_performed: u8,
    pub mutation_polls: u8,
    pub restore_polls: u8,
    pub value_before: u32,
    pub probe_value: u32,
    pub mutation_observed: u32,
    pub restored_value: u32,
    pub snapshot_fingerprint: u64,
    pub c21_transaction_fingerprint: u64,
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

impl C22State {
    pub const EMPTY: Self = Self {
        amd_present: false,
        navi48: false,
        profile_verified: false,
        exact_domain_live: false,
        c19_snapshot_verified: false,
        c20_exact_bases_ready: false,
        c21_identity_verified: false,
        c21_target_reused: false,
        target_revalidated: false,
        target_dword_offset: 0,
        target_byte_offset: 0,
        bar5_ready: false,
        memory_decode_before_on: false,
        memory_decode_after_on: false,
        bus_master_before_off: false,
        bus_master_after_off: false,
        transaction_eligible: false,
        mutation_attempted: false,
        mutation_verified: false,
        restore_attempted: false,
        restore_verified: false,
        restore_retry_used: false,
        writes_performed: 0,
        mutation_polls: 0,
        restore_polls: 0,
        value_before: 0,
        probe_value: 0,
        mutation_observed: 0,
        restored_value: 0,
        snapshot_fingerprint: 0,
        c21_transaction_fingerprint: 0,
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

static STATE: SpinLock<C22State> = SpinLock::new(C22State::EMPTY);

/// Derive the only mutation value C22 may write. This changes exactly one bit:
/// clear the least-significant set bit, or set bit zero if the value is zero.
pub const fn derive_one_bit_probe(original: u32) -> u32 {
    if original == 0 { 1 } else { original & original.wrapping_sub(1) }
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

fn fingerprint(
    device: u16,
    revision: u8,
    target: u64,
    before: u32,
    probe: u32,
    observed: u32,
    restored: u32,
    writes: u8,
    mutation_polls: u8,
    restore_polls: u8,
) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for byte in device.to_le_bytes().into_iter()
        .chain([revision])
        .chain(target.to_le_bytes())
        .chain(before.to_le_bytes())
        .chain(probe.to_le_bytes())
        .chain(observed.to_le_bytes())
        .chain(restored.to_le_bytes())
        .chain([writes, mutation_polls, restore_polls]) {
        h ^= u64::from(byte);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn self_test() -> Result<(), &'static str> {
    if K14C22_ABI_VERSION != 1
        || RADEON_C22_MMIO_BAR_INDEX != 5
        || RADEON_C22_PAGE_BYTES != 4096
        || RADEON_C22_MAX_MUTATION_POLLS != 32
        || RADEON_C22_MAX_RESTORE_POLLS != 32
        || RADEON_C22_MAX_MMIO_WRITES != 3
        || !RADEON_C22_ONE_BIT_SCRATCH_MUTATION_ALLOWED
        || RADEON_C22_ARBITRARY_MMIO_WRITES_ALLOWED
        || RADEON_C22_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C22_BAR_RESIZE_ALLOWED
        || RADEON_C22_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C22_COMMAND_SUBMIT_ALLOWED
        || RADEON_C22_BUS_MASTER_ALLOWED
        || RADEON_C22_CALLER_SUPPLIED_VALUE_ALLOWED
        || RADEON_C22_CALLER_SUPPLIED_ADDRESS_ALLOWED {
        return Err("K14.C22 reversible-mutation/fail-closed constants invalid");
    }
    for original in [0u32, 1, 2, 3, 0x55aa_55aa, 0x8000_0000, 0xffff_fffe] {
        let probe = derive_one_bit_probe(original);
        if probe == original || probe == u32::MAX {
            return Err("K14.C22 one-bit probe derivation self-test failed");
        }
        let changed = original ^ probe;
        if changed.count_ones() != 1 {
            return Err("K14.C22 probe changes more than one bit");
        }
    }
    if fingerprint(0x7550, 0xc0, 0x9040, 0x55aa55aa, derive_one_bit_probe(0x55aa55aa),
        derive_one_bit_probe(0x55aa55aa), 0x55aa55aa, 2, 1, 1) == 0 {
        return Err("K14.C22 transaction fingerprint self-test failed");
    }
    Ok(())
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

pub fn initialize(
    allocator: &mut FrameAllocator<'_>,
    kernel_cr3: u64,
) -> Result<C22State, &'static str> {
    self_test()?;
    let c19 = native_gpu_c19::state();
    let c21 = native_gpu_c21::state();
    let mut s = C22State {
        amd_present: c21.amd_present,
        navi48: c21.navi48,
        profile_verified: c21.profile_verified,
        exact_domain_live: c21.exact_domain_live,
        c19_snapshot_verified: c21.c19_snapshot_verified,
        c20_exact_bases_ready: c21.c20_exact_bases_ready,
        c21_identity_verified: c21.transaction_verified,
        c21_target_reused: c21.gc_segment1_resolved && c21.gc_segment0_crosschecked
            && c21.target_dword_offset != 0 && c21.target_byte_offset != 0,
        target_dword_offset: c21.target_dword_offset,
        target_byte_offset: c21.target_byte_offset,
        snapshot_fingerprint: c21.snapshot_fingerprint,
        c21_transaction_fingerprint: c21.transaction_fingerprint,
        device_id: c21.device_id,
        revision: c21.revision,
        ..C22State::EMPTY
    };

    serial::println(format_args!(
        "[C22RV] reversible GFX12 scratch mutation: source=C21_exact_SCRATCH_REG0 derivation=clear_lowest_set_bit_or_set_bit0_if_zero caller_value=false caller_address=false"
    ));
    serial::println(format_args!(
        "[C22PG] reversible-write policy: require=C21_identity_verified+same_checksum_snapshot+same_exact_target+BAR5+memory_decode_on+bus_master_off; mutation_polls<={} restore_polls<={} max_writes={} arbitrary=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master_enable=false",
        RADEON_C22_MAX_MUTATION_POLLS, RADEON_C22_MAX_RESTORE_POLLS, RADEON_C22_MAX_MMIO_WRITES
    ));
    serial::println(format_args!(
        "[C22TX] reversible transaction contract: read_original -> derive_one_bit_probe -> write_probe -> exact_probe_readback -> write_original -> exact_restore_readback -> one_final_restore_retry_if_needed -> recheck_memory_decode_and_bus_master"
    ));

    if !s.amd_present {
        serial::println(format_args!(
            "[C22HW] GFX12 SCRATCH_REG0 reversible mutation: present=false qemu_deferred=true target=false attempted=false mutation=false restored=false fallback=true"
        ));
    } else {
        let c21_ready = s.navi48 && s.profile_verified && s.exact_domain_live
            && s.c19_snapshot_verified && s.c20_exact_bases_ready
            && s.c21_identity_verified && s.c21_target_reused
            && s.snapshot_fingerprint != 0
            && s.snapshot_fingerprint == c19.snapshot_fingerprint
            && s.c21_transaction_fingerprint != 0;
        if !c21_ready {
            serial::println(format_args!(
                "[C22HW] GFX12 SCRATCH_REG0 reversible mutation: present=true devid={:#06x} C21_identity={} target={} snapshot_match={} attempted=false reason=C21_promotion_gate_not_ready fallback=true",
                s.device_id, s.c21_identity_verified, s.c21_target_reused,
                s.snapshot_fingerprint != 0 && s.snapshot_fingerprint == c19.snapshot_fingerprint
            ));
        } else {
            let target = native_gpu_c19::with_verified_snapshot(native_gpu_c21::resolve_gfx12_scratch_reg0)
                .ok_or("K14.C22 C19 verified snapshot unavailable")??;
            s.target_revalidated = target.valid
                && target.target_dword_offset == s.target_dword_offset
                && target.target_byte_offset == s.target_byte_offset
                && target.gc_segment1_base_dwords == c21.gc_segment1_base_dwords;
            if !s.target_revalidated {
                return Err("K14.C22 live target no longer matches frozen C21 proof");
            }

            let function = selected_function();
            let command_before = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_before_on = command_before & (1 << 1) != 0;
            s.bus_master_before_off = command_before & (1 << 2) == 0;
            if !s.memory_decode_before_on {
                return Err("K14.C22 Radeon PCI memory decoding is disabled");
            }
            if !s.bus_master_before_off {
                return Err("K14.C22 Radeon bus mastering unexpectedly enabled before reversible mutation");
            }

            let bar = pci::memory_bar_base(function, RADEON_C22_MMIO_BAR_INDEX)
                .ok_or("K14.C22 Radeon BAR5 unavailable")?;
            s.bar5_ready = true;
            s.transaction_eligible = true;

            let page_off = s.target_byte_offset & !(RADEON_C22_PAGE_BYTES - 1);
            let in_page = s.target_byte_offset & (RADEON_C22_PAGE_BYTES - 1);
            if in_page + 4 > RADEON_C22_PAGE_BYTES {
                return Err("K14.C22 reviewed target crosses MMIO page boundary");
            }
            let phys = bar.checked_add(page_off).ok_or("K14.C22 MMIO physical overflow")?;
            let virt = paging::map_kernel_mmio(allocator, kernel_cr3, phys, RADEON_C22_PAGE_BYTES)?;
            let p = (virt + in_page) as *mut u32;

            s.value_before = unsafe { core::ptr::read_volatile(p) };
            if s.value_before == u32::MAX {
                return Err("K14.C22 reviewed SCRATCH_REG0 read returned all ones");
            }
            s.probe_value = derive_one_bit_probe(s.value_before);
            if s.probe_value == s.value_before || (s.value_before ^ s.probe_value).count_ones() != 1 {
                return Err("K14.C22 derived probe violates one-bit mutation contract");
            }

            s.mutation_attempted = true;
            unsafe { core::ptr::write_volatile(p, s.probe_value) };
            s.writes_performed = 1;
            let (mutation_ok, mutation_observed) = poll_exact(
                p, s.probe_value, &mut s.mutation_polls, RADEON_C22_MAX_MUTATION_POLLS
            );
            s.mutation_observed = mutation_observed;
            s.mutation_verified = mutation_ok;

            // Restore is mandatory whether the probe readback succeeded or not.
            s.restore_attempted = true;
            unsafe { core::ptr::write_volatile(p, s.value_before) };
            s.writes_performed = 2;
            let (restore_ok, restored) = poll_exact(
                p, s.value_before, &mut s.restore_polls, RADEON_C22_MAX_RESTORE_POLLS
            );
            s.restored_value = restored;
            s.restore_verified = restore_ok;

            if !s.restore_verified {
                s.restore_retry_used = true;
                unsafe { core::ptr::write_volatile(p, s.value_before) };
                s.writes_performed = 3;
                let mut retry_polls = 0u8;
                let (retry_ok, retry_value) = poll_exact(
                    p, s.value_before, &mut retry_polls, RADEON_C22_MAX_RESTORE_POLLS
                );
                s.restore_polls = s.restore_polls.saturating_add(retry_polls);
                s.restored_value = retry_value;
                s.restore_verified = retry_ok;
            }

            let command_after = pci::read_u16(function.bus, function.device, function.function, 0x04);
            s.memory_decode_after_on = command_after & (1 << 1) != 0;
            s.bus_master_after_off = command_after & (1 << 2) == 0;
            if !s.memory_decode_after_on || !s.bus_master_after_off {
                return Err("K14.C22 PCI decode/bus-master safety changed during reversible mutation");
            }
            if !s.restore_verified {
                return Err("K14.C22 could not verify restoration of original SCRATCH_REG0 value");
            }
            if !s.mutation_verified {
                return Err("K14.C22 one-bit scratch mutation did not read back exactly");
            }

            s.transaction_fingerprint = fingerprint(
                s.device_id, s.revision, s.target_dword_offset, s.value_before,
                s.probe_value, s.mutation_observed, s.restored_value,
                s.writes_performed, s.mutation_polls, s.restore_polls,
            );
            serial::println(format_args!(
                "[C22HW] GFX12 SCRATCH_REG0 reversible mutation: present=true navi48=true devid={:#06x} target_dwords={:#x} before={:#010x} probe={:#010x} observed={:#010x} restored={:#010x} writes={} mutation_polls={} restore_polls={} retry={} attempted=true mutation=true restored_ok=true memory_decode_before=true memory_decode_after=true bus_master_before=false bus_master_after=false fingerprint={:#018x} fallback=true",
                s.device_id, s.target_dword_offset, s.value_before, s.probe_value,
                s.mutation_observed, s.restored_value, s.writes_performed,
                s.mutation_polls, s.restore_polls, s.restore_retry_used,
                s.transaction_fingerprint
            ));
        }
    }

    if s.mutation_verified && (!s.transaction_eligible || !s.mutation_attempted
        || !s.restore_attempted || !s.restore_verified || !s.target_revalidated
        || !s.bar5_ready || !s.memory_decode_before_on || !s.memory_decode_after_on
        || !s.bus_master_before_off || !s.bus_master_after_off || !s.c21_identity_verified) {
        return Err("K14.C22 mutation qualified without every reversible-write safety gate");
    }
    if s.writes_performed > RADEON_C22_MAX_MMIO_WRITES
        || s.arbitrary_mmio_write_enabled || s.mm_index_fallback_used || s.bar_resize_used
        || s.firmware_upload_enabled || s.command_submit_enabled || s.radeon_bus_master_enabled
        || RADEON_C22_ARBITRARY_MMIO_WRITES_ALLOWED || RADEON_C22_MM_INDEX_FALLBACK_ALLOWED
        || RADEON_C22_BAR_RESIZE_ALLOWED || RADEON_C22_FIRMWARE_UPLOAD_ALLOWED
        || RADEON_C22_COMMAND_SUBMIT_ALLOWED || RADEON_C22_BUS_MASTER_ALLOWED
        || RADEON_C22_CALLER_SUPPLIED_VALUE_ALLOWED || RADEON_C22_CALLER_SUPPLIED_ADDRESS_ALLOWED {
        return Err("K14.C22 capability escaped bounded reversible SCRATCH_REG0 contract");
    }

    serial::println(format_args!(
        "[C22RD] K14.C22 reversible-write ready: amd_present={} navi48={} profile={} domain={} C19_verified={} C20_ready={} C21_identity={} C21_target={} revalidated={} target_dwords={:#x} BAR5={} memdecode_before={} eligible={} attempted={} mutation={} restore_attempted={} restored={} retry={} writes={} mutation_polls={} restore_polls={} memdecode_after={} bus_master_before_off={} bus_master_after_off={} snapshot_fp={:#018x} c21_fp={:#018x} tx_fp={:#018x} arbitrary=false caller_value=false caller_address=false MM_INDEX=false BAR_resize=false firmware=false submit=false bus_master=false fallback=true",
        s.amd_present, s.navi48, s.profile_verified, s.exact_domain_live,
        s.c19_snapshot_verified, s.c20_exact_bases_ready, s.c21_identity_verified,
        s.c21_target_reused, s.target_revalidated, s.target_dword_offset,
        s.bar5_ready, s.memory_decode_before_on, s.transaction_eligible,
        s.mutation_attempted, s.mutation_verified, s.restore_attempted,
        s.restore_verified, s.restore_retry_used, s.writes_performed,
        s.mutation_polls, s.restore_polls, s.memory_decode_after_on,
        s.bus_master_before_off, s.bus_master_after_off, s.snapshot_fingerprint,
        s.c21_transaction_fingerprint, s.transaction_fingerprint
    ));
    *STATE.lock() = s;
    Ok(s)
}

pub fn state() -> C22State { *STATE.lock() }

pub fn packed_status() -> u64 {
    let s = state();
    let mut v = (u64::from(s.device_id) << 40)
        | (u64::from(s.revision) << 32)
        | (u64::from(s.writes_performed) << 24)
        | (u64::from(s.restore_polls.min(255)) << 16);
    for (bit, on) in [
        s.amd_present,             // bit 0
        s.navi48,                  // bit 1
        s.profile_verified,        // bit 2
        s.exact_domain_live,       // bit 3
        s.c21_identity_verified,   // bit 4
        s.c21_target_reused,       // bit 5
        s.target_revalidated,      // bit 6
        s.transaction_eligible,    // bit 7
        s.mutation_attempted,      // bit 8
        s.mutation_verified,       // bit 9
        s.restore_attempted,       // bit 10
        s.restore_verified,        // bit 11
        s.bus_master_before_off,   // bit 12
        s.bus_master_after_off,    // bit 13
        s.fallback_armed,          // bit 14
    ].into_iter().enumerate() {
        if on { v |= 1u64 << bit; }
    }
    v
}
