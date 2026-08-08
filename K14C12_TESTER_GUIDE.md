# K14.C12 Tester Guide

Run on Fedora:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c12-qemu-trusted-bases.sh
```

QEMU has no native Radeon, so the safe deferred path is expected. Required markers include `[C12BS]`, `[C12RM]`, `[C12LR]`, `[C12HW]`, `[C12RD]`, `[C12OK]`, DISPLAYD C12 banner/deferred message, stable userspace handoff, C12 alive/qualification, and intentional BSP halt.

Expected result:

`Titanweave K14.C12 trusted-base/live-read runtime qualification PASSED.`

Do not interpret QEMU success as bare-metal Radeon qualification. On physical hardware C12 may only read after all exact-domain/profile/base/BAR5 gates are true; all write-side paths remain fenced.
