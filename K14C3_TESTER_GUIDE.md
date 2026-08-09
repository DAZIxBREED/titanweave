# Titanweave K14.C3 Tester Guide

## QEMU regression/staging qualification
```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c3-qemu-radeon-staging.sh
```

QEMU has no native Radeon model. The correct result is `amd_present=false`, all vendor-write/firmware/command gates false, and VirtIO-GPU/GOP fallback retained.

## Host-side inventory
Before bare-metal testing, the Fedora host can be inventoried read-only:
```bash
./tools/host-k14c3-amd-inventory.sh
```

## Bare-metal preflight
Before any future Radeon MMIO test, boot on the target AMD machine and confirm the log reports an AMD display candidate, ForgeBus ownership, bus mastering disabled, IVRS/AMD-Vi discovery, and `actual_domain=false`. C3 must stop there until the actual requester receives a persistent AMD-Vi translated domain.

Never enable Radeon bus mastering manually to bypass this gate.
