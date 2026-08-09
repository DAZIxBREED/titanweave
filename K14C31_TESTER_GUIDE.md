# Titanweave K14.C31 Tester Guide

```bash
cd ~/Downloads
rm -rf titanweave-kernel-k14c31-integrated
unzip titanweave-kernel-k14c31-integrated.zip
cd titanweave-kernel-k14c31-integrated
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14c31-qemu-graphics-compute-execution.sh
```

QEMU has no native Radeon. Qualification therefore proves the executable Titanweave reference backend over real owned GTT/C30 framebuffer resources, queues, command streams and fences. It does not claim physical Radeon shader execution.
