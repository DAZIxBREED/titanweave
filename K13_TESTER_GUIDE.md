# Titanweave K13.C Fedora/QEMU Test Guide

This source tree must be tested separately from frozen K13.B.

## 1. Enter K13.C

```bash
cd ~/Downloads/titanweave-kernel-k13c-integrated
```

## 2. Static/inherited validation

```bash
./tools/validate-source.sh
```

Expected final line:

```text
Titanweave K1-K13.C integrated source validation passed (K13.C runtime qualification still required).
```

## 3. Compile

```bash
PROFILE=debug ./tools/build.sh
```

Do not run `cargo fix` on the kernel warning set while runtime qualification is
active.

## 4. QEMU presentation qualification

```bash
./tools/run-k13c-qemu-gpu.sh
```

K13.C intentionally retains `-vga std` as the K12 GOP recovery display while the
secondary modern VirtIO-GPU carries the K13 presentation path.

Important new markers:

```text
[PACE] compositor pacing contract:
[FBCK] presentation watchdog policy:
[PRES] triple-buffered compositor scanout online:
[DMG ] dirty-region GPU uploads verified:
[PFEN] fence-verified presentation complete:
[FBCK] GOP fallback remains armed after accelerated presentation: true
[GCOMP] K13.C compositor presentation ready:
[GPRE] K13.C buffered presentation ready:
[USER] [displayd] K13.C display/compositor service online
[UPRS] DISPLAYD present:
[USER] [displayd] K13.C capability-mediated buffered present verified
[KERN] K13.C alive:
[QUAL] K13.C presentation runtime reached intentional post-userspace halt
[HALT] BSP halted intentionally
```

## 5. Re-check a saved log

```bash
./tools/check-k13c-serial-log.sh build/k13c-serial.log
```

Target:

```text
Titanweave K13.C presentation/runtime qualification PASSED.
```
