# K14.C14 Tester Guide

## Fedora/QEMU qualification

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c14-qemu-write-readiness.sh
```

Expected final result:

```text
Titanweave K14.C14 write-promotion-readiness runtime qualification PASSED.
```

QEMU must report that no physical Radeon is present and that write-promotion readiness remains deferred.

## Safety expectation

C14 must not enable Radeon MMIO writes, firmware upload, command submission, or PCI bus mastering. A physical Radeon may become *eligible* only after every C14 prerequisite passes, but actual promotion remains disabled.
