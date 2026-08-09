# Titanweave K14.C30 Tester Guide

## Fedora/QEMU qualification

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c30-integrated
unzip titanweave-kernel-k14c30-integrated.zip
cd titanweave-kernel-k14c30-integrated

./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c30-qemu-basic-display-engine.sh
```

QEMU's UEFI GOP framebuffer is the real C30 scanout backend. The test allocates two GTT scanout surfaces, renders two distinct frames, presents both into the live framebuffer and verifies the scanout changed. QEMU has no physical Radeon, so native DCN programming and physical HPD remain explicitly unclaimed.

Expected ending:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C29OK] K14.C29 Radeon rings+queues+fences+DMA:
PASS  [C30ED] EDID/mode engine:
PASS  [C30CN] connector topology:
PASS  [C30SC] double-buffer scanout:
PASS  [C30MS] atomic modeset:
PASS  [C30HP] hotplug engine:
PASS  [C30DC] DCN401 resource authority:
PASS  [C30PG] display authority:
PASS  [C30RD] K14.C30 complete basic display ready:
PASS  [C30OK] K14.C30 complete basic display engine:
PASS  [USER] [displayd] K14.C30 complete basic display engine online
PASS  [USER] [displayd] K14.C30 QEMU/GOP backend verified: EDID parser, connector/CRTC/plane model, double-buffered GTT scanout, live framebuffer page flips, atomic mode rollback, and hotplug bookkeeping operational
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio
PASS  [KERN] K14.C30 alive:
PASS  [QUAL] K14.C30 complete-basic-display-engine runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C30 complete-basic-display-engine runtime qualification PASSED.
QEMU stopped after intentional kernel halt (raw exit status: 0)
```
