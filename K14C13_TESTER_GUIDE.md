# K14.C13 Tester Guide

Run:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c13-qemu-physical-proof.sh
```

QEMU must report C13PV/C13HW/C13RD/C13OK, DISPLAYD's C13 banner and no-physical-
Radeon message, stable userspace handoff, the C13 QUAL marker, and intentional
halt.

A later bare-metal qualification run on a supported Radeon must additionally
produce C12R0/C12R1/C12R2 plus C13SN with `proof=true`, while bus mastering,
MMIO writes, firmware upload, and command submission remain disabled.
