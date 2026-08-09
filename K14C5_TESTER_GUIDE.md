# Titanweave K14.C5 Tester Guide

## QEMU regression

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c5-qemu-amdvi-page-tables.sh
```

Expected final line:

```text
Titanweave K14.C5 AMD-Vi page-table/runtime qualification PASSED.
```

QEMU has no physical Radeon, so `[C5HW]` must report `present=false` and no persistent Radeon domain or bus mastering may be promoted.

## Read-only AMD host inventory

```bash
./tools/host-k14c5-amdvi-radeon-inventory.sh
```

This only inventories PCI/IOMMU/IVRS state. It does not unbind the Fedora amdgpu driver or write GPU/IOMMU registers.
