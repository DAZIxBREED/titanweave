# K14.C6 Tester Guide

## QEMU regression qualification

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c6-qemu-amdvi-live.sh
```

Expected QEMU behavior: no physical Radeon, so AMD-Vi physical programming is deferred while all prior K14 milestones remain intact. The final line must be `Titanweave K14.C6 live AMD-Vi/runtime qualification PASSED.`

## Bare-metal host inventory

From Fedora, before any future destructive test:

```bash
./tools/host-k14c6-amdvi-live-inventory.sh
```

This helper is read-only. Do not arm `AMDVI_C6_ENABLE_BARE_METAL_PROGRAMMING` until the inventory and IVRS ownership mapping have been reviewed.
