# Titanweave K14.C1 Tester Guide

Run on Fedora:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c1-qemu-native-binding.sh
```

Expected K14.C1 markers include `[AMDB]`, `[NVRM]`, `[NSEL]`, `[NBND]`, `[NDOM]`, and `[NCF ]`. QEMU normally reports `candidates=0`; that is correct because QEMU does not emulate AMD/Intel/NVIDIA hardware. The qualified VirtIO-GPU and GOP paths must remain active.
