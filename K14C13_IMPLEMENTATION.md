# Titanweave K14.C13 — Physical Radeon Read-Proof Qualification

K14.C13 freezes K14.C12 and turns its bounded Radeon status reads into a bare-metal qualification proof. It does not enable any new destructive GPU capability.

## Qualification proof
- exact C9 Radeon profile and PCI identity must already be verified;
- C12 must have completed exactly three whitelisted status reads;
- GRBM_STATUS, GRBM_CHIP_REVISION, and SDMA0_STATUS_REG evidence must be valid;
- an all-ones-only MMIO response is rejected as absent/unimplemented hardware;
- PCI command bus-master bit is re-read and must remain clear;
- a deterministic FNV-1a evidence fingerprint is emitted for log comparison;
- writes, firmware upload, command submission, and Radeon bus mastering remain disabled.

## Navi48
Navi48 remains fail-closed until trusted AMD IP-discovery data supplies the required GC/SDMA base addresses. C13 does not substitute guessed offsets.

QEMU validates the policy/runtime path with physical proof intentionally deferred.
