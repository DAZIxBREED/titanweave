# K14.C19 tester guide

## Fedora/QEMU qualification

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
K13_DISPLAY=none ./tools/run-k14c19-qemu-physical-snapshot.sh
```

QEMU has no native Radeon, so the expected qualification path is a safe defer. The serial checker requires C19 policy, aperture, hardware-defer, readiness, userspace, kernel qualification, and intentional halt markers.

Expected final line:

```text
Titanweave K14.C19 physical-snapshot runtime qualification PASSED.
```

## Bare-metal behavior

On a supported physical Radeon, C19 will attempt the direct read only when all earlier K14 safety gates are live, PCI memory decoding is enabled, bus mastering is OFF, BAR5 scratch/TMR information is usable, ACPI MCFG exposes ECAM, BAR0 has a Resizable BAR entry, and the complete discovery TMR fits inside the current BAR0 aperture. Any missing prerequisite causes a fail-closed defer rather than a guessed access.
