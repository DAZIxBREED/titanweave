# Titanweave K14.C23 — Post-Restore Persistence and Dual-Probe Stability

K14.C23 advances from the frozen, QEMU-qualified K14.C22 baseline without widening Radeon write authority.

## Goal

C22 proved one bounded reversible one-bit mutation of the exact checksum-backed GFX12 `SCRATCH_REG0` target and exact restoration of its original value. C23 asks a narrower reliability question before any future register expansion: does that restored value persist, and can the same reviewed scratch register survive two distinct reversible mutations in sequence?

## Physical transaction gate

On supported Navi48 bare metal, C23 requires all frozen C22 prerequisites plus:

- C22 mutation readback verified;
- C22 original-value restoration verified;
- C22 exact target revalidation retained;
- nonzero C22 transaction fingerprint;
- the same C19 checksum-qualified discovery snapshot;
- the same exact C21/C22 GFX12 `SCRATCH_REG0` target;
- BAR5 available;
- PCI memory decoding enabled;
- Radeon bus mastering clear immediately before the C23 transaction.

C23 first reads `SCRATCH_REG0` and requires exact equality with C22's recorded original value. This is the cross-milestone persistence proof.

## Two distinct probe cycles

Probe A reuses C22's deterministic one-bit derivation. Probe B is independently derived from the same original value and must:

- differ from the original;
- differ from Probe A;
- differ by exactly one bit from the original;
- never be `0xffffffff`;
- never come from userspace/caller input.

Each cycle is:

1. write the internally-derived probe;
2. poll at most 32 reads for exact probe equality;
3. write the original value back;
4. poll at most 32 reads for exact restoration;
5. if restoration fails, permit exactly one final restore write/retry for recovery;
6. fail closed if exact mutation or restoration cannot be proved.

After Cycle A, C23 performs an additional exact read of the original value before Cycle B. This provides an inter-cycle persistence check.

Maximum physical MMIO stores are six: three per cycle only when both cycles require their single recovery restore retry. Normal successful operation uses four stores total.

## Still forbidden

C23 does not authorize:

- any Radeon register other than the exact C21/C22 `SCRATCH_REG0` target;
- caller-selected register addresses;
- caller-selected write values;
- arbitrary MMIO writes;
- MM_INDEX/MM_DATA fallback;
- BAR resizing;
- firmware upload;
- GPU command submission;
- Radeon bus-master enable.

QEMU contains no physical Radeon, so the hardware transaction must remain explicitly deferred while the source contract, ABI, userspace reporting, fallback path, and qualification markers are exercised.
