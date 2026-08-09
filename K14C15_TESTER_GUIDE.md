# K14.C15 Tester Guide

## Fedora/QEMU qualification

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
K13_DISPLAY=none ./tools/run-k14c15-qemu-controlled-write.sh
```

QEMU has no physical Radeon, so the controlled identity-write path must remain deferred while the complete runtime reaches userspace and the intentional halt.

Expected final line:

```text
Titanweave K14.C15 controlled-write runtime qualification PASSED.
```

## Physical path

On supported bare metal, C15 may attempt the identity-write transaction only when C14 reports every write-side prerequisite complete. The log should include `[C15RB]` and show equal before/after PCI Command values with bus mastering false both before and after. Any mismatch, unexpected bus-master state, or rollback requirement fails qualification.
