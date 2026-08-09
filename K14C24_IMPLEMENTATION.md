# Titanweave K14.C24 — Reversible Multi-Bit SCRATCH_REG0 Pattern

K14.C24 advances from the frozen, QEMU-qualified K14.C23 baseline without widening Radeon register authority.

## Goal

C23 proved post-restore persistence and two distinct one-bit probe/restore cycles on the exact checksum-backed GFX12 `SCRATCH_REG0`. C24 increases only write-data complexity: one internally-derived four-bit pattern is written to that same target, read back exactly, and then the original value is restored exactly.

## Physical gate

On supported Navi48 bare metal, C24 requires frozen C23 dual-cycle verification, C23 exact-target revalidation, a nonzero C23 transaction fingerprint, the same C19 checksum-qualified discovery snapshot, BAR5, PCI memory decoding enabled, and Radeon bus mastering disabled. Before writing, C24 reads the target and requires the C23-restored original value to still be present.

## Deterministic four-bit pattern

No caller controls the value. Normally C24 computes `pattern = original ^ 0x0000000f`, changing exactly four bits. If that would produce `0xffffffff`, it instead uses `original ^ 0x000000f0`. The result always differs from the original by exactly four bits and is never all ones.

## Transaction contract

1. Verify C23-restored value persistence.
2. Derive the internal four-bit pattern.
3. Write the pattern.
4. Poll at most 32 reads for exact pattern equality.
5. Mandatorily write the original value back.
6. Poll at most 32 reads for exact restoration.
7. If restoration fails, permit one final restore write/retry only for recovery.
8. Recheck PCI Command: memory decode must remain on and Radeon bus mastering off.

Maximum MMIO stores: three. Normal success uses two.

## Still forbidden

No new Radeon register, caller-selected address/value, arbitrary MMIO, MM_INDEX/MM_DATA fallback, BAR resizing, firmware upload, GPU command submission, or Radeon bus-master enable is authorized. QEMU has no physical Radeon, so qualification exercises the explicit deferred path and does not claim a physical store occurred.
