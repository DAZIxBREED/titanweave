# Titanweave K14.C29 Tester Guide

## Fedora/QEMU qualification

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c29-integrated
unzip titanweave-kernel-k14c29-integrated.zip
cd titanweave-kernel-k14c29-integrated

./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c29-qemu-rings-queues-fences-dma.sh
```

QEMU contains no physical Radeon. The C29 qualification therefore executes the real GTT ring/queue/fence/typed-DMA software path on actual Titanweave-mapped pages and validates the exact SDMA packet/register definitions while the physical SDMA programming/bus-master path remains safely deferred.

Expected ending:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C28OK] K14.C28 Radeon memory+firmware+recovery:
PASS  [C29RG] SDMA ring:
PASS  [C29QU] submission queue:
PASS  [C29FN] timeline fence:
PASS  [C29DM] typed SDMA DMA:
PASS  [C29SD] GFX12 SDMA0 queue0 authority:
PASS  [C29PG] execution authority:
PASS  [C29RD] K14.C29 rings+queues+fences+DMA ready:
PASS  [C29OK] K14.C29 Radeon rings+queues+fences+DMA:
PASS  [USER] [displayd] K14.C29 Radeon rings, queues, timeline fences, and typed DMA subsystem online
PASS  [USER] [displayd] K14.C29 no physical Radeon in QEMU; GTT ring, FIFO queue, SDMA packet codec, timeline fence, and owned-memory DMA executor qualified while physical SDMA remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio
PASS  [KERN] K14.C29 alive:
PASS  [QUAL] K14.C29 rings-queues-fences-dma runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C29 rings-queues-fences-dma runtime qualification PASSED.
QEMU stopped after intentional kernel halt (raw exit status: 0)
```
