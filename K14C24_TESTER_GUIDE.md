# Titanweave K14.C24 Tester Guide

## Fedora/QEMU qualification

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c24-integrated
unzip titanweave-kernel-k14c24-integrated.zip
cd titanweave-kernel-k14c24-integrated

./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c24-qemu-multi-bit-pattern.sh
```

The runner defaults to GTK and automatically terminates QEMU after Titanweave emits `[HALT] BSP halted intentionally`. QEMU contains no physical Radeon, so the expected hardware path is safely deferred while the source/self-test/ABI/userspace/fallback contract is qualified.

Expected ending:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C23OK] K14.C23 GFX12 SCRATCH_REG0 persistence/dual-probe stability gate:
PASS  [C24PT] reversible multi-bit pattern:
PASS  [C24PG] multi-bit-write policy:
PASS  [C24TX] reversible pattern contract:
PASS  [C24HW] GFX12 SCRATCH_REG0 reversible multi-bit pattern:
PASS  [C24RD] K14.C24 multi-bit-pattern ready:
PASS  [C24OK] K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate:
PASS  [USER] [displayd] K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate online
PASS  [USER] [displayd] K14.C24 no physical Radeon in QEMU; reversible multi-bit pattern transaction remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [KERN] K14.C24 alive:
PASS  [QUAL] K14.C24 reversible-multi-bit-pattern runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C24 reversible-multi-bit-pattern runtime qualification PASSED.
QEMU stopped after intentional kernel halt (raw exit status: 0)
```

## Bare-metal boundary

Only a later supported Navi48 bare-metal run can prove the physical four-bit mutation/readback/restoration transaction. QEMU qualification must not be recorded as physical MMIO proof.
