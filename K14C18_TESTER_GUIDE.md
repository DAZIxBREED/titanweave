# Titanweave K14.C18 Tester Guide

## Fedora / QEMU qualification

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
K13_DISPLAY=none ./tools/run-k14c18-qemu-snapshot-verification.sh
```

QEMU does not expose a physical Radeon. The expected result is therefore a
successful checksum-engine/self-test path with physical discovery acquisition
explicitly deferred.

Expected final line:

```text
Titanweave K14.C18 snapshot-verification runtime qualification PASSED.
```

## Expected milestone markers

- `[C18CK] discovery checksum verifier:`
- `[C18PG] snapshot acquisition policy:`
- `[C18HW] AMD discovery snapshot:`
- `[C18RD] K14.C18 snapshot-verification gate ready:`
- `[C18OK] K14.C18 AMD discovery snapshot-verification gate:`

This QEMU qualification does **not** constitute a physical Radeon TMR/VRAM read.
