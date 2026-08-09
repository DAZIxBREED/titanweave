# K14.C20 Tester Guide

Run source validation and the GUI-enabled QEMU qualification:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
K13_DISPLAY=gtk ./tools/run-k14c20-qemu-exact-ip-bases.sh
```

`gtk` is also the runner default, so the last command may be run without `K13_DISPLAY=gtk`.

Expected QEMU markers include `C20IP`, `C20PG`, `C20HW`, `C20RD`, `C20OK`, the DISPLAYD C20 banner/deferred message, `RECV`, `KERN`, `QUAL`, and `HALT`.

A QEMU pass qualifies the C20 framework/deferred runtime path. It does not claim that a bare-metal Radeon snapshot was present or that physical GC/SDMA bases were resolved during the VM run.
