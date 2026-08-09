# K14.C17 tester guide

Run:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
K13_DISPLAY=none ./tools/run-k14c17-qemu-ip-discovery.sh
```

QEMU has no native Radeon, so C17 must report the parser online and the live discovery fetch safely deferred, then reach the intentional qualification halt.
