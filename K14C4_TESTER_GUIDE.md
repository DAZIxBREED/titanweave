# Titanweave K14.C4 Tester Guide

## QEMU regression / contract qualification

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c4-qemu-radeon-domain-gate.sh
```

Expected C4 markers include:

- `[C4IV] AMD-Vi exact-requester gate:`
- `[C4DM] Radeon domain policy:`
- `[C4AP] Radeon aperture promotion policy:`
- `[C4HW] physical Radeon domain bind:`
- `[C4RD] K14.C4 Radeon exact-domain gate ready:`
- `[C4NF] K14.C4 Radeon exact-domain qualification ready:`

In QEMU the correct result is `amd_present=false`, `domain_live=false`, and
`bus_master=false`.

## Fedora host inventory before bare metal

This helper is read-only:

```bash
./tools/host-k14c4-amdvi-radeon-inventory.sh
```

It reports AMD display/audio PCI functions, IOMMU groups, driver binding, BAR
resources, and whether the host exposes IVRS/AMD-IOMMU evidence. It does not
unbind devices or modify PCI command bits.
