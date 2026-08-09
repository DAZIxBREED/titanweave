# Titanweave K14.A Tester Guide

## Fedora/QEMU gate

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
./tools/run-k14-qemu-native-gpu.sh
```

The QEMU gate intentionally continues to use the qualified K13.D VirtIO-GPU
path. QEMU normally has no AMD/Intel/NVIDIA display adapter, so K14.A should
report zero native candidates and safely defer native activation.

Expected K14.A markers include:

```text
[NDRV] native backend contract self-test:
[NGPU] native adapter probe:
[IOMQ] native DMA admission:
[NFAL] native activation deferred; K13 VirtIO-GPU and K12 GOP fallback remain armed
[NATF] K14.A native GPU prerequisite foundation ready:
[USER] [displayd] K14.A native GPU activation deferred; qualified VirtIO/GOP fallback retained
[KERN] K14.A alive:
[QUAL] K14.A native-GPU foundation runtime reached intentional post-userspace halt
[HALT] BSP halted intentionally
```

Then run:

```bash
./tools/check-k14-serial-log.sh build/k14-serial.log
```

## Bare-metal note

If this same tree is booted on a machine with AMD/Intel/NVIDIA graphics, K14.A
may log `[NBAR]` and a nonzero native adapter count. That is discovery evidence
only. `activation_ready` must remain false until K14.B implements and qualifies
real translated DMA.


## K14.C6
Live AMD-Vi hardware-programming boundary added; QEMU must remain fail-closed and bare-metal activation is separately gated.
