# Titanweave Completion Matrix

- K11: qualified/frozen
- K12: qualified/frozen
- K13.A-D: qualified/frozen
- K14.A: qualified/frozen
- K14.B: qualified/frozen
- K14.C1: qualified/frozen
- K14.C2: qualified/frozen
- K14.C3: source-integrated; QEMU staging qualification pending
- K14.D: pending


## K14.C12
Trusted Radeon IP-base sources and first bounded live status-read path. QEMU qualification pending. Write-side Radeon paths remain fenced.

## K14.C21
Qualified/frozen. Exact generated GFX12 `SCRATCH_REG0` target rebind and bounded identity-write gate. Intentional-HALT QEMU harness termination is frozen into the baseline.

## K14.C22
Qualified/frozen. First bounded reversible non-identity GFX12 `SCRATCH_REG0` mutation: internally derived one-bit probe, exact readback, mandatory original-value restoration, one bounded restore retry, and Radeon bus-master fencing.

## K14.C23
Qualified/frozen. Requires the C22-restored value to persist, then performs two distinct internally-derived one-bit probe/readback/restore cycles on the exact same checksum-backed `SCRATCH_REG0` target. Fedora/QEMU qualification passed through `[C23OK]`, userspace handoff, `[QUAL]`, and intentional `[HALT]`, with automatic QEMU termination. Arbitrary MMIO writes, caller-selected values/addresses, firmware upload, command submission, BAR resizing, MM_INDEX fallback, and bus-master enable remain fenced.

## K14.C24
Qualified/frozen. Requires frozen C23 stability, then performs one internally-derived four-bit pattern/readback/restore cycle on the exact same checksum-backed `SCRATCH_REG0` target. Fedora/QEMU qualification passed through `[C24OK]`, userspace handoff, `[QUAL]`, and intentional `[HALT]`, with automatic QEMU termination. Arbitrary MMIO writes, caller-selected values/addresses, firmware upload, command submission, BAR resizing, MM_INDEX fallback, and bus-master enable remain fenced.
