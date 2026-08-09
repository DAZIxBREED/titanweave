# TITAN//WEAVE — WeaveCore K14 Native GPU Bring-up

K14 forks from frozen, qualified GPU/runtime milestones. K13 remains the generic ForgeGraphics/VirtIO/GOP safety baseline; K14 adds native physical-GPU ownership without weakening those fallbacks.

## Current slice: K14.C5

K14.A established read-only native GPU discovery and fail-closed DMA admission. K14.B proved real Intel VT-d translated DMA and IOVA revocation. K14.C1 now adds AMD-first native GPU selection, ForgeBus ownership, non-destructive BAR inventory, backend-neutral VRAM/GTT ownership, and DISPLAYD binding visibility.

K14.C1 deliberately does **not** enable native GPU bus mastering or write vendor MMIO. K14.B's translation qualification is short-lived; K14.C2 must bind and retain a translated domain for the exact physical GPU requester before Radeon firmware/command-ring work begins.

See `docs/architecture/K14.md`, `K14_STATUS.md`, `K14C5_IMPLEMENTATION.md`, and `K14C5_TESTER_GUIDE.md`.

## Validate and test

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c1-qemu-native-binding.sh
```

Frozen K14.B remains separate and must not be overwritten.


## K14.C2
Persistent translated-domain lifetime proof plus AMD-first firmware/ring bring-up contract. QEMU uses EDU only as an IOMMU requester surrogate; native Radeon MMIO/DMA remains fail-closed.

## K14.C3
Radeon bare-metal bring-up staging: AMD IP/firmware/MMIO safety contract with exact-requester translated-domain gating. Native Radeon command submission remains fenced pending bare-metal domain qualification.

## K14.C4

K14.C4 adds the exact Radeon requester / AMD-Vi qualification gate. QEMU exercises the fail-closed contract; bare-metal AMD qualification is required before the physical Radeon can receive a persistent translated domain or bus-master authority.

## K14.C5

K14.C5 adds the AMD-Vi page-table engine foundation: pinned device table, second-level root, command/event buffers, exact Radeon requester DTE image, and default-deny fault-state plumbing. Physical AMD-Vi register programming remains a bare-metal qualification gate.


## K14.C6
Live AMD-Vi hardware-programming boundary added; QEMU must remain fail-closed and bare-metal activation is separately gated.

### K14.C9
Verified Radeon device-profile and live safe PCI identity-read foundation. Native GPU MMIO access remains fail-closed until exact per-IP register whitelists are verified.


## K14.C10
Per-IP Radeon MMIO whitelist engine and guarded live-read activation gate added; physical offsets remain fail-closed until exact IP-specific review.


### Active milestone: K14.C11
C11 adds source-reviewed Radeon status-register definitions and an explicit IP-relative register-address resolver. Physical MMIO reads remain fenced until trusted IP base discovery is available.


## K14.C12
Trusted Radeon IP-base sources and first bounded live status-read path. QEMU qualification pending. Write-side Radeon paths remain fenced.

### K14.C13
Current development milestone: physical Radeon read-proof qualification. The write side remains fenced; C13 must pass Fedora/QEMU qualification before freeze.

### Current kernel milestone
K14.C14 adds the controlled Radeon write-promotion readiness gate on top of the frozen C13 physical-read-proof framework. Destructive GPU capabilities remain fenced.


### Current kernel milestone: K14.C15
C15 introduces the first real write-side transaction without touching Radeon MMIO: a width-correct PCI Command identity write/readback with rollback and bus-master-off verification. GPU register writes and initialization remain fenced pending a separately reviewed target.
