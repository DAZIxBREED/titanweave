# Titanweave K14.C26 — Final K14 Reviewed MMIO Allowlist + SCRATCH_REG1 Read Proof

K14.C26 is the **final K14 completion milestone**. It advances from frozen K14.C25 without opening another write transaction.

## Why C26 ends K14

C21-C25 established a progressively stronger exact-target write/readback/restore proof on GFX12 `SCRATCH_REG0`. Continuing to add mutation patterns would no longer materially improve the native-GPU prerequisite foundation. C26 therefore converts those proofs into a reusable reviewed MMIO boundary and adds one second independently-addressed register under read-only authority. Broader Radeon bring-up moves to K15 after C26 runtime qualification.

## Source-reviewed second target

AMD-generated GFX12 register definitions identify:

- `regSCRATCH_REG0 = 0x2040`, `BASE_IDX = 1` (frozen C21 target)
- `regSCRATCH_REG1 = 0x2041`, `BASE_IDX = 1` (C26 second target)

The GFX12 AMDGPU path also initializes RLCG access-control slots from both `regSCRATCH_REG0` and `regSCRATCH_REG1`. C26 reuses C21's checksum-qualified C19/C20 parser/base proof and computes `SCRATCH_REG1` from the same GC base slot 1. It requires REG1 to be exactly one dword/four bytes after REG0 and to remain distinct.

## Final K14 allowlist

The kernel materializes two reviewed entries:

1. `SCRATCH_REG0`: `FrozenReversibleProbe` — historical bounded C21-C25 authority only.
2. `SCRATCH_REG1`: `ReadOnly` — C26 may perform bounded volatile reads but **zero MMIO stores**.

No caller can supply a register address or value.

## Physical read-proof contract

On a supported physical Navi48 path C26 requires:

1. Frozen C25 dual-pattern proof and exact REG0 target revalidation.
2. Same nonzero checksum-qualified C19 discovery snapshot fingerprint.
3. Same GFX12 GC base slot 1 used by C21-C25.
4. Exact REG1 resolution (`0x2041`, base index 1).
5. REG0/REG1 distinctness and exact one-dword adjacency.
6. Radeon PCI memory decode enabled and bus mastering disabled.
7. BAR5 present and aligned one-page mapping of the exact REG1 dword.
8. Four bounded `read_volatile` samples, all rejecting `0xffffffff` open-bus/all-ones behavior.
9. Recheck PCI memory decode remains enabled and bus mastering remains disabled.
10. Require C26 MMIO writes performed = **0**.

## Still forbidden

C26 does not authorize REG1 writes, arbitrary MMIO, caller-selected address/value, MM_INDEX/MM_DATA fallback, BAR resizing, firmware upload, GPU command submission, or Radeon bus-master enable.

QEMU has no physical Radeon. The QEMU path intentionally defers physical REG1 access while qualifying source integration, ABI/userspace reporting, safety fences, and the final K14 completion markers.
