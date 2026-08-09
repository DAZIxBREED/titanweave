# Titanweave K14.C28 Runtime Qualification

Status: **QUALIFIED / FROZEN**

Fedora/QEMU runtime qualification completed successfully on 2026-08-09. The run passed `[BOOT]`, frozen `[C27OK]`, all C28 gates (`[C28ME]`, `[C28FW]`, `[C28RC]`, `[C28PG]`, `[C28RD]`, `[C28OK]`), both C28 `displayd` userspace lines, stable userspace handoff (`[RECV]`), the frozen K14 continuation marker (`[K14FOUND]`), kernel liveness (`[KERN]`), `[QUAL]`, and intentional `[HALT]`. The halt-aware runner terminated QEMU cleanly with raw exit status 0.

Observed qualification ending:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C27OK] K14.C27 complete Radeon driver core:
PASS  [C28ME] Radeon memory manager:
PASS  [C28FW] Radeon firmware manager:
PASS  [C28RC] Radeon recovery manager:
PASS  [C28PG] memory/firmware/recovery authority:
PASS  [C28RD] K14.C28 memory+firmware+recovery ready:
PASS  [C28OK] K14.C28 Radeon memory+firmware+recovery:
PASS  [USER] [displayd] K14.C28 Radeon memory, firmware staging, and recovery subsystem online
PASS  [USER] [displayd] K14.C28 no physical Radeon in QEMU; real GTT allocation/mapping/reclaim plus firmware parser/CRC and watchdog recovery logic qualified while hardware firmware staging is deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio
PASS  [KERN] K14.C28 alive:
PASS  [QUAL] K14.C28 memory-firmware-recovery runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C28 memory-firmware-recovery runtime qualification PASSED.
QEMU stopped after intentional kernel halt (raw exit status: 0)
```

## Qualification boundary

QEMU contains no physical Radeon. This freezes the C28 source/self-test/runtime/userspace safe-defer path, real Titanweave memory-management behavior exercised by the QEMU configuration, firmware parser/validation/staging logic, and software recovery logic. It does **not** claim physical Radeon firmware silicon upload, physical ASIC reset, GPU page-table programming, Radeon DMA/bus mastering, rings/queues/fences, command submission, or physical GPU interrupt programming. Those capabilities remain outside C28 authority.
