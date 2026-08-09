# Titanweave K14.C8 — Radeon ASIC/IP identification and safe-register-read gate

K14.C8 extends the qualified K14.C7 read-only Radeon aperture with a fail-closed ASIC/IP profile layer.

## Contract

- A PCI vendor/device/revision tuple is identity evidence, **not** permission to touch arbitrary MMIO.
- Register access requires an explicit `VerifiedAsicProfile` grounded in reviewed hardware documentation for the exact ASIC/IP revision.
- Every readable register is represented by `SafeRegisterDescriptor`; any descriptor marked as having read side effects is rejected.
- Unknown ASICs remain fenced.
- The initial C8 profile table is intentionally empty. No Radeon PCI ID or register offset is guessed.
- GPU register writes, firmware upload, command submission and Radeon bus mastering remain disabled.

## Promotion sequence

C7 exact-domain + supervisor read-only aperture → verified ASIC profile → GC/GMC/SDMA/DCN IP manifest → side-effect-free read whitelist → firmware requirement resolution → GMC/GTT readiness → later bare-metal read activation.

QEMU has no native Radeon, so C8 runtime qualification proves the deferred/fail-closed path and all inherited C7 fallbacks.
