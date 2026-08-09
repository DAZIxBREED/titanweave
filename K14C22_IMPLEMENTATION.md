# Titanweave K14.C22 — Bounded reversible GFX12 scratch mutation

K14.C22 freezes K14.C21 as the qualified exact-target/identity-write baseline and advances one deliberately narrow step: the first **non-identity** Radeon MMIO mutation, restricted to the same generated GFX12 `SCRATCH_REG0` target and required to restore the original value before qualification can succeed.

## Promotion boundary

C22 may run the physical mutation only when frozen C21 has already proved, in the same boot:

- verified Navi48 PCI identity;
- persistent translated DMA domain;
- checksum-qualified C19 discovery snapshot;
- exact C20 GFX12 base resolution;
- exact generated `regSCRATCH_REG0=0x2040`, `BASE_IDX=1` target;
- C21 target cross-check and successful identity write;
- BAR5 register aperture;
- PCI memory decoding enabled;
- Radeon bus mastering disabled.

C22 re-opens only the already verified C19 snapshot, resolves the C21 target again, and requires the live target to match C21 exactly before any store.

## Deterministic one-bit probe

No caller can select the address or data value. C22 derives the probe internally:

- if the original value is nonzero: `probe = original & (original - 1)`, clearing exactly the least-significant set bit;
- if the original value is zero: `probe = 1`.

The probe is therefore always different from the original, changes exactly one bit, and never becomes `0xffffffff`.

## Transaction contract

On an eligible physical Navi48 device:

1. read original `SCRATCH_REG0` and reject an all-ones response;
2. derive the one-bit probe;
3. perform one probe write;
4. poll at most 32 reads for exact probe equality;
5. **always** write the original value back;
6. poll at most 32 reads for exact restoration;
7. if restoration is not verified, perform one final bounded restore write and another bounded readback window;
8. require the original value restored before any successful qualification;
9. re-read PCI Command and require memory decode still enabled and bus mastering still disabled.

At most three MMIO stores are permitted: probe, mandatory restore, and one final restore retry.

## Still fenced

C22 does not permit caller-supplied MMIO addresses or values, arbitrary MMIO writes, MM_INDEX/MM_DATA fallback, BAR resizing, firmware upload, GPU command submission, or Radeon bus-master enable.

QEMU has no physical Radeon, so runtime qualification exercises the explicit deferred path plus self-tests, ABI, userspace reporting, and the frozen VirtIO-GPU/GOP fallback. Bare-metal qualification remains the authority for the physical mutation/restore transaction.

## Harness behavior

C22 inherits the frozen C21 intentional-HALT harness fix. The QEMU runner watches the serial stream for `[HALT] BSP halted intentionally`, terminates QEMU itself, then runs the serial qualification checker. A successful Titanweave halt therefore no longer looks like a QEMU lockup.
