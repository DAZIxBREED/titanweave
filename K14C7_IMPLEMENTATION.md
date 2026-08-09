# Titanweave K14.C7 — Controlled Radeon MMIO / Firmware Discovery

K14.C7 follows the qualified K14.C6 AMD-Vi programming boundary. Its purpose is to cross onto the Radeon side without crossing the destructive-write boundary.

## Implemented

- exact C6 persistent-domain prerequisite for any Radeon MMIO mapping;
- supervisor-only, uncached, NX **read-only** kernel MMIO mapper;
- first-memory-BAR selection from the existing non-destructive AMD BAR inventory;
- 4 KiB probe-aperture mapping contract;
- PCI vendor/device/revision identity capture;
- VBIOS locator / ASIC-IP / firmware-manifest discovery plan;
- GMC/GTT readiness plan;
- C7 ABI/status query and DISPLAYD reporting;
- QEMU fail-closed qualification with VirtIO/GOP fallback.

## Deliberately not enabled

C7 does not dereference Radeon registers yet. MMIO register reads can have device-specific side effects and therefore remain a separate gate after ASIC/IP identity is known. CPU stores, PCI bus mastering, firmware upload, and GPU command submission remain disabled.

## Safety invariant

A Radeon MMIO page may only be mapped when K14.C6 reports both a live exact-requester AMD-Vi domain and read-only-MMIO promotion. The mapping itself has no PAGE_WRITABLE bit.
