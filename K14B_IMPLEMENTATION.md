# K14.B Implementation Record — Hardware-Translated DMA

K14.B forks from the frozen, QEMU-qualified K14.A baseline. It does not modify
or weaken the K13 VirtIO-GPU/GOP fallback path.

## What this checkpoint proves

K14.B adds a live Intel VT-d translation qualification path. The QEMU test adds
one QEMU EDU PCI DMA endpoint (`1234:11e8`) whose only purpose is to generate a
small deterministic bus-master transfer through the guest IOMMU.

The qualification sequence is deliberately strict:

1. K11 DMAR discovery must already have identified an Intel VT-d unit.
2. The EDU function is claimed through ForgeBus while bus mastering is still
   disabled.
3. Two ForgeBus-owned DMA pages are allocated.
4. A legacy VT-d root table, context table and 39/48-bit second-level page walk
   are built from kernel-owned physical frames.
5. Only two IOVAs are mapped: a read-only source and a write-only destination.
6. The VT-d root pointer is installed, context/IOTLB caches are invalidated,
   and translation is enabled.
7. Only after translation is live is PCI bus mastering enabled for EDU.
8. A RAM -> EDU internal buffer -> RAM round trip must reproduce the exact
   source bytes.
9. The destination IOVA is removed and the IOTLB globally invalidated.
10. A second device-to-RAM DMA to that revoked IOVA must leave destination RAM
    unchanged. VT-d fault status is logged as additional evidence.
11. Bus mastering is disabled before the context is cleared.
12. Context and IOTLB caches are invalidated, translation is disabled, and the
    ForgeBus DMA pages are reclaimed.

This is a real hardware-translation test against QEMU's emulated VT-d engine;
it is not merely a software page-table walker test.

## Native GPU admission boundary

A successful K14.B qualification promotes the native-IOMMU readiness state from
`PolicyOnly` to `HardwareTranslated`. It does **not** make a physical GPU ready
to bus-master. K14.B intentionally reports `device_domain_bound=false`; K14.C
must claim a concrete AMD/Intel/NVIDIA adapter and install its own translated
per-device domain before native bus mastering may be authorized.

## Intel and AMD scope

The live QEMU qualification is Intel VT-d because Q35 exposes DMAR/Intel IOMMU.
The existing IVRS/AMD-Vi discovery path remains fail-closed and is exposed to
K14 through the same backend handoff, but K14.B does not claim a live AMD-Vi
hardware qualification without an IVRS bare-metal target. That must be proven on
real AMD hardware before an AMD native GPU is allowed to advance into K14.C.

## Current deliberate boundary

K13's already-qualified VirtIO-GPU transport still runs with
`iommu_platform=off`. K14.B proves the hardware translation primitive in an
isolated short window and then tears it down before userspace. Persistent
`VIRTIO_F_ACCESS_PLATFORM` migration and persistent native-GPU translated DMA
are later integration gates, not claims made by this checkpoint.
