# Titanweave K14.C28 Tester Guide

## Fedora/QEMU qualification

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c28-integrated
unzip titanweave-kernel-k14c28-integrated.zip
cd titanweave-kernel-k14c28-integrated

./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c28-qemu-memory-firmware-recovery.sh
```

QEMU contains no physical Radeon. The run nevertheless executes real C28 system-memory work: contiguous GTT page allocation, cacheable kernel mapping, volatile write/readback, unmapping and physical reclaim. It also executes the firmware-header/CRC self-test and watchdog/recovery state-machine tests. Physical VRAM reservation, actual firmware-file staging for the Radeon, and its live recovery interrupt route are hardware-dependent and remain explicitly deferred in QEMU.

Expected ending:

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

## Physical firmware preparation

Titanweave's current bootstrap FAT32 reader is 8.3-name based. For a later physical-Radeon C28 run, place the required AMD GFX12 firmware files in `C:\\SYSTEM\\FIRMWARE` using the short aliases documented in `K14C28_IMPLEMENTATION.md`. C28 validates and stages them but does not upload them to the GPU.
