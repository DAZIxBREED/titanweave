# Titanweave K14.C25 — Dual Multi-Bit SCRATCH_REG0 Pattern Stability

K14.C25 advances from the frozen, QEMU-qualified K14.C24 baseline without widening Radeon register authority.

## Goal

C24 proved one deterministic reversible four-bit pattern on the exact checksum-backed GFX12 `SCRATCH_REG0` target. C25 increases repetition and bit-position coverage, not register authority: it requires the C24-restored value to persist, then executes two distinct internally-derived four-bit pattern/readback/restore cycles on that same target with an explicit persistence check between cycles.

## Pattern derivation

No caller controls address or data. Cycle A normally toggles bits 0..3 (`original ^ 0x0000000f`), falling back to bits 8..11 only if the normal result would be `0xffffffff`. Cycle B normally toggles bits 4..7 (`original ^ 0x000000f0`), falling back to bits 12..15 only if necessary. Both patterns always differ from the original by exactly four bits, never equal `0xffffffff`, and are distinct from each other.

## Transaction contract

1. Require frozen C24 pattern verification, exact restoration, exact-target revalidation, nonzero C24 transaction fingerprint, and the same checksum-qualified C19 snapshot.
2. Re-resolve and cross-check the exact GFX12 `SCRATCH_REG0` target.
3. Require PCI memory decode on and Radeon bus mastering off.
4. Read the target and require the C24-restored original value to persist.
5. Cycle A: write pattern A, bounded exact readback, mandatory original restore, bounded exact restore, one recovery restore retry only if needed.
6. Re-read the target and require the original value to persist between cycles.
7. Cycle B: write distinct pattern B, bounded exact readback, mandatory original restore, bounded exact restore, one recovery restore retry only if needed.
8. Recheck PCI Command: memory decode remains on and Radeon bus mastering remains off.

Normal success uses four MMIO stores. Worst-case bounded recovery permits six stores total.

## Still forbidden

No new Radeon register, caller-selected address/value, arbitrary MMIO, MM_INDEX/MM_DATA fallback, BAR resizing, firmware upload, GPU command submission, or Radeon bus-master enable is authorized. QEMU has no physical Radeon and therefore qualifies the explicit deferred path only.
