# Titanweave K14.C28 — Memory + Firmware + Recovery

Status: **IMPLEMENTED / SOURCE-VALIDATED / RUNTIME QUALIFICATION PENDING**

K14.C28 is the third large milestone in the locked K14 Radeon roadmap. It starts only from frozen K14.C27 and implements working memory ownership, firmware validation/staging, and software recovery mechanisms. No stub or placeholder subsystem is permitted.

## Radeon memory manager

C28 adds `radeon_memory.rs` with actual ownership and reclamation:

- BAR0 VRAM aperture reservations use the existing splitting/coalescing `IovaAllocator`; reservations can be allocated, aligned, freed, and reused without writing VRAM.
- GTT backing is allocated as real contiguous pages from `FrameAllocator`.
- `paging.rs` now provides a dedicated 1 GiB supervisor-only, NX, normal-cacheable kernel DMA aperture at `0xffff_ff80_0000_0000`.
- GTT pages are mapped into that aperture, zeroed, CPU-written/read back, unmapped, and returned to the physical allocator.
- Every Radeon memory object has an owner, object id, physical backing where applicable, kernel virtual mapping where applicable, and a reserved GPU virtual address.
- A 1 TiB GPU-VA reservation space begins at `0x0000_0001_0000_0000`. Allocation/free is real, but hardware GPU page tables are deliberately not installed in C28.
- C28 leaves a pinned 16 KiB control/recovery-journal GTT object alive after qualification so later C29 code consumes an existing real allocation rather than a fake descriptor.

C28 does **not** claim that a GPU-VA reservation is executable by the GPU before C29 installs the required translation/submission machinery.

## Firmware manager

`radeon_firmware.rs` implements the AMD common firmware header and validates:

- complete file size,
- header size,
- microcode offset/size bounds,
- IEEE CRC-32 of the microcode payload,
- SHA-256 equality after copy into GTT staging memory.

Validated firmware is copied into real Radeon-owned GTT objects and pinned. The current FAT32 VFS is short-name based, so C28 recognizes these Titanweave boot-volume aliases under `C:\SYSTEM\FIRMWARE`:

- `GFXRLC.BIN`
- `GFXME.BIN`
- `GFXMEC.BIN`
- `GFXPFP.BIN`
- `GFXMES.BIN`
- `GFXMES1.BIN`
- `GFXIMU.BIN`
- `GFXTOC.BIN`

On physical Radeon hardware, at least one validated firmware image must be staged for the C28 physical gate to close. QEMU has no native Radeon, so it qualifies the real parser/CRC implementation and the hardware-deferred staging path. Firmware is **not uploaded into GPU silicon** in C28.

## Recovery and interrupt activation

`radeon_recovery.rs` uses Titanweave's operational `DriverWatchdog` and the live C27 driver object. It can:

1. register the actual Radeon ForgeBus driver with the watchdog,
2. activate the already-owned Titanweave software interrupt route required for recovery dispatch,
3. quiesce the live driver,
4. mask the route during a recovery transaction,
5. transition the driver into a faulted state,
6. unpin/unmap/free caller-owned C28 memory objects,
7. run the C27 reset coordinator,
8. return the driver to Online,
9. reactivate its owned software interrupt route.

The watchdog's `None -> Ping -> Restart -> Quarantine` behavior is executed by self-test. C28 does not claim to program Radeon IH/MSI hardware or perform a physical ASIC reset.

## Locked C29 boundary

Still forbidden in C28:

- Radeon bus-master enable,
- device DMA engines,
- GPU page-table installation,
- command rings or queues,
- command submission,
- physical Radeon interrupt/MSI programming,
- firmware upload into GPU silicon,
- unreviewed MMIO writes.

Those execution mechanisms belong to locked milestone **K14.C29 — Rings + queues + fences + DMA**.
