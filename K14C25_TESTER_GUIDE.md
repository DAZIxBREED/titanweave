# Titanweave K14.C25 Tester Guide

## Fedora/QEMU qualification

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c25-integrated
unzip titanweave-kernel-k14c25-integrated.zip
cd titanweave-kernel-k14c25-integrated

./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c25-qemu-dual-multi-bit-pattern.sh
```

The runner defaults to GTK and automatically terminates QEMU after Titanweave emits `[HALT] BSP halted intentionally`. QEMU contains no physical Radeon, so the expected physical transaction is safely deferred while the source/self-test/ABI/userspace/fallback contract is qualified.

Expected ending:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C24OK] K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate:
PASS  [C25DP] dual multi-bit patterns:
PASS  [C25PG] dual-pattern policy:
PASS  [C25TX] dual-pattern stability contract:
PASS  [C25HW] GFX12 SCRATCH_REG0 dual multi-bit pattern stability:
PASS  [C25RD] K14.C25 dual-pattern ready:
PASS  [C25OK] K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate:
PASS  [USER] [displayd] K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate online
PASS  [USER] [displayd] K14.C25 no physical Radeon in QEMU; dual multi-bit pattern stability transaction remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [KERN] K14.C25 alive:
PASS  [QUAL] K14.C25 dual-multi-bit-pattern runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C25 dual-multi-bit-pattern runtime qualification PASSED.
QEMU stopped after intentional kernel halt (raw exit status: 0)
```

## Bare-metal boundary

A QEMU PASS does not prove the physical dual-pattern MMIO sequence. Bare-metal qualification on a supported Navi48/RX 9070-class device remains the authority for the actual writes and restorations.
