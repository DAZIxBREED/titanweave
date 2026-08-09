# K14.C3 Implementation Record — Radeon Bare-Metal Bring-up Staging

K14.C3 forks from the frozen, QEMU-qualified K14.C2 baseline.

## Added
- AMD IP bring-up ordering contract: PSP → SMU → GMC → IH → SDMA → GC → DCN.
- Firmware manifest contract for VBIOS, PSP, SMU, GC, SDMA, and scheduler/MES-family roles.
- Fail-closed Radeon MMIO/doorbell policy.
- Explicit AMD-Vi/IVRS readiness gate for AMD-host bare-metal testing.
- DISPLAYD K14.C3 status query.
- QEMU qualification proves that no fake Radeon is promoted and that VirtIO-GPU/GOP fallbacks remain armed.

## Deliberate boundary
C3 does not yet write Radeon vendor registers, upload firmware, enable Radeon bus mastering, or submit commands. Those operations require the exact physical Radeon requester to own a persistent hardware-translated domain. On the intended AMD bare-metal target this also requires real AMD-Vi page-table programming and IVRS qualification; the prior Intel VT-d/QEMU proof is not silently treated as equivalent.
