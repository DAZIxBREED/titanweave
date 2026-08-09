# K14.C10 tester guide

Run:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c10-qemu-mmio-whitelist.sh
```

QEMU has no physical Radeon, so the expected result is a deferred physical-MMIO path and a successful qualification halt. Required C10 markers are `[C10WL]`, `[C10RD]`, `[C10HW]`, `[C10NF]`, `[C10OK]`, the DISPLAYD C10 banner, `[KERN] K14.C10 alive:`, `[QUAL] K14.C10 ...`, and `[HALT]`.
