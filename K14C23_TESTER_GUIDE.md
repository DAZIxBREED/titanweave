# Titanweave K14.C23 Tester Guide

## Fedora/QEMU qualification

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c23-integrated
unzip titanweave-kernel-k14c23-integrated.zip
cd titanweave-kernel-k14c23-integrated

./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c23-qemu-dual-probe-stability.sh
```

The runner defaults to GTK display and automatically terminates QEMU after the intentional Titanweave `[HALT] BSP halted intentionally` success marker, so a qualified halt must not appear as a manual lockup.

QEMU does not provide a physical Radeon. The expected path proves the complete source/self-test/ABI/userspace/deferred contract while preserving VirtIO-GPU/GOP fallback. It does not claim that the physical C23 MMIO stores executed on Navi48 hardware.

Expected ending:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C22OK] K14.C22 reversible GFX12 SCRATCH_REG0 mutation gate:
PASS  [C23PS] post-restore persistence gate:
PASS  [C23PG] dual-probe policy:
PASS  [C23TX] stability transaction contract:
PASS  [C23HW] GFX12 SCRATCH_REG0 dual-probe stability:
PASS  [C23RD] K14.C23 stability ready:
PASS  [C23OK] K14.C23 GFX12 SCRATCH_REG0 persistence/dual-probe stability gate:
PASS  [USER] [displayd] K14.C23 GFX12 SCRATCH_REG0 persistence and dual-probe stability gate online
PASS  [USER] [displayd] K14.C23 no physical Radeon in QEMU; dual-probe stability transaction remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [KERN] K14.C23 alive:
PASS  [QUAL] K14.C23 dual-probe-stability runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C23 dual-probe-stability runtime qualification PASSED.
QEMU stopped after intentional kernel halt (raw exit status: 0)
```

## Bare-metal boundary

A later explicit bare-metal qualification on a supported Navi48 device is required to prove cross-milestone scratch-value persistence and both physical mutation/restore cycles. QEMU qualification must never be recorded as proof of those physical stores.
