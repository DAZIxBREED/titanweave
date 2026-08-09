# Titanweave K14.C22 tester guide

## Fedora/QEMU qualification

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c22-integrated
unzip titanweave-kernel-k14c22-integrated.zip
cd titanweave-kernel-k14c22-integrated

./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c22-qemu-reversible-scratch-mutation.sh
```

The runner defaults to `K13_DISPLAY=gtk`. It automatically terminates QEMU after Titanweave emits its intentional `[HALT]` marker, so no manual `fuser`/`kill` step should be necessary after a successful run.

QEMU has no physical Radeon. The expected C22 path is therefore safely deferred, while the C22 value-derivation self-test, exact-target dependency, userspace ABI, fallback path, and qualification plumbing are exercised.

Expected ending:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C21OK] K14.C21 reviewed GFX12 target rebind/identity-write gate:
PASS  [C22RV] reversible GFX12 scratch mutation:
PASS  [C22PG] reversible-write policy:
PASS  [C22TX] reversible transaction contract:
PASS  [C22HW] GFX12 SCRATCH_REG0 reversible mutation:
PASS  [C22RD] K14.C22 reversible-write ready:
PASS  [C22OK] K14.C22 reversible GFX12 SCRATCH_REG0 mutation gate:
PASS  [USER] [displayd] K14.C22 bounded reversible GFX12 SCRATCH_REG0 mutation gate online
PASS  [USER] [displayd] K14.C22 no physical Radeon in QEMU; reversible scratch mutation remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [KERN] K14.C22 alive:
PASS  [QUAL] K14.C22 reversible-scratch-mutation runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C22 reversible-scratch-mutation runtime qualification PASSED.
QEMU stopped after intentional kernel halt (raw exit status: 0)
```

## Bare-metal boundary

A QEMU PASS is not evidence that the physical one-bit MMIO mutation occurred. Bare-metal C22 qualification on a supported Navi48/RX 9070-class device must show both exact probe readback and exact restoration with Radeon bus mastering remaining off.
