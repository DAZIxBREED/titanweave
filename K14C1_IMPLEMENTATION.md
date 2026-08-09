# K14.C1 Implementation Record — Native GPU Ownership Foundation

K14.C1 forks from the frozen, QEMU-qualified K14.B baseline. It starts the AMD-first native GPU backend without claiming that QEMU emulates a Radeon.

## What K14.C1 adds

- AMD-first native backend classifier and ForgeBus ownership path.
- Fail-closed clearing of any firmware-left PCI bus-master bit before AMD ownership.
- Read-only non-destructive BAR inventory.
- Bounded ForgeBus software DMA domain for an actual AMD candidate.
- Backend-neutral VRAM/GTT buffer ownership self-test.
- Native GPU selection policy: AMD first, then Intel, then NVIDIA.
- A new DISPLAYD binding-status syscall.
- Explicit separation between a *qualified translation engine* and a *persistent domain bound to this GPU requester*.

## Deliberate boundary

K14.B tears its VT-d qualification domain down before normal services continue. Therefore K14.C1 does not enable native GPU bus mastering or vendor MMIO writes. K14.C2 must create a persistent translated domain for the selected GPU requester and only then may the AMD backend advance toward firmware/command-ring bring-up.

QEMU has no native Radeon/Intel/NVIDIA GPU model, so the QEMU gate honestly expects the native candidate count to be zero while proving all inherited K14.B/K13 paths and the new ownership/memory contracts.
