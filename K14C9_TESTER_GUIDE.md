# Titanweave K14.C9 Tester Guide

Run the normal source validation/build, then the QEMU qualification:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c9-qemu-verified-profiles.sh
```

QEMU has no native Radeon, so the expected path is deferred/fail-closed. Required C9 markers include `[C9PF]`, `[C9PR]`, `[C9IP]`, `[C9HW]`, `[C9RD]`, and `[C9NF]`, followed by the intentional halt.

Expected finish:

```text
Titanweave K14.C9 verified-profile/runtime qualification PASSED.
```

A later bare-metal run on a supported Radeon may perform the PCI identity reads, but C9 still must not read Radeon MMIO registers or enable bus mastering.
