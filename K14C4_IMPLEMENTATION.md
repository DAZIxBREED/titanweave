# Titanweave K14.C4 — Radeon Exact-Requester / AMD-Vi Qualification Gate

K14.C4 forks from the qualified K14.C3 baseline. It is the first stage that ties
an actual AMD display-function BDF to the AMD-Vi/IVRS admission path.

## Contract

K14.C4 requires, in order:

1. An AMD display-class PCI function is selected by the K14.C1 backend.
2. ForgeBus already owns that exact function and the PCI bus-master bit is off.
3. The BDF is converted to the exact IOMMU Requester ID (RID).
4. IVRS/AMD-Vi is present on an AMD bare-metal machine.
5. A persistent translated-domain implementation for that RID must be proven.
6. Only then may a read-only Radeon MMIO aperture be promoted.
7. MMIO writes, firmware upload, command submission, and bus mastering remain
   separate later gates.

The current retained `amd_vi.rs` remains an IVRS/default-deny backend contract;
K14.C4 does **not** falsely label it a production hardware page-table engine.
Accordingly `persistent_domain_live` stays false until that hardware path is
implemented and qualified on real AMD hardware.

## QEMU qualification

QEMU does not expose a Radeon. The QEMU C4 milestone therefore verifies:

- all K14.C3 regressions remain green;
- the exact-requester/AMD-Vi contract initializes;
- no surrogate EDU or VirtIO device can promote the physical-Radeon bit;
- bus mastering, MMIO writes, firmware upload, and command submission remain off;
- DISPLAYD can observe the C4 state;
- K13 VirtIO-GPU and K12 GOP fallback remain available.

## Bare-metal continuation

The next implementation step after C4 qualification is the real AMD-Vi
second-level page-table engine for the selected Radeon RID, followed by a
read-only Radeon register-aperture probe. No destructive Radeon programming is
part of this source slice.
