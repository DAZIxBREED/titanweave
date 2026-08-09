# Titanweave K14.C27 Tester Guide

## Fedora/QEMU qualification

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c27-integrated
unzip titanweave-kernel-k14c27-integrated.zip
cd titanweave-kernel-k14c27-integrated

./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c27-qemu-radeon-driver-core.sh
```

The QEMU runner automatically terminates after `[HALT] BSP halted intentionally`.

QEMU does not contain a physical Radeon. C27 therefore qualifies the real software driver core, ForgeBus/resource policy model, permanent reviewed-MMIO policy, lifecycle/error/reset machinery, real interrupt-router handler dispatch self-test, userspace ABI and explicit physical-hardware defer path. Physical Radeon ownership/MMIO read execution remains a bare-metal proof.

Expected ending includes:

```text
PASS  [C26OK] K14.C26 final reviewed GFX12 MMIO allowlist/read-only completion gate:
PASS  [C27DV] Radeon driver core:
PASS  [C27RS] Radeon resource ownership/topology:
PASS  [C27MM] Radeon reviewed MMIO service:
PASS  [C27IR] Radeon interrupt core:
PASS  [C27ER] Radeon error/reset coordinator:
PASS  [C27PG] driver-core authority:
PASS  [C27RD] K14.C27 complete driver-core ready:
PASS  [C27OK] K14.C27 complete Radeon driver core:
PASS  [USER] [displayd] K14.C27 complete Radeon driver core online
PASS  [USER] [displayd] K14.C27 no physical Radeon in QEMU; operational driver-core software paths qualified and physical ownership/MMIO route safely deferred
PASS  [K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio
PASS  [KERN] K14.C27 alive:
PASS  [QUAL] K14.C27 complete-radeon-driver-core runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C27 complete-radeon-driver-core runtime qualification PASSED.
```
