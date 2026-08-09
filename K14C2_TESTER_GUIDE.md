# K14.C2 Tester Guide

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c2-qemu-native-domain.sh
```

Expected new markers: `[PDOM]`, `[PDRV]`, `[AFWP]`, `[ARING]`, `[ASIC]`, `[C2RD]`, `[C2NF]`. QEMU must keep `actual_gpu_domain=false` and `bus_master=false` for the native GPU because no Radeon is emulated.
