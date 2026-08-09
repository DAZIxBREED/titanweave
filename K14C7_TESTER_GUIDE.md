# K14.C7 Tester Guide

Run:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c7-qemu-radeon-discovery.sh
```

QEMU has no physical Radeon, so the correct result is that C7 reports `present=false` and keeps Radeon MMIO, register access, firmware upload, command submission and bus mastering fenced.

Expected final line:

```text
Titanweave K14.C7 Radeon discovery/runtime qualification PASSED.
```
