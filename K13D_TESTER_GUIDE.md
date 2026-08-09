# Titanweave K13.D Fedora/QEMU Test Guide

K13.D must be tested separately from the frozen K13.C archive.

## 1. Enter K13.D

```bash
cd ~/Downloads/titanweave-kernel-k13d-integrated
```

## 2. Validate the inherited and K13.D source gates

```bash
./tools/validate-source.sh
```

Expected final line:

```text
Titanweave K1-K13.D integrated source validation passed (K13.D runtime qualification still required).
```

## 3. Compile

```bash
PROFILE=debug ./tools/build.sh
```

Do not run `cargo fix` on the kernel warning set while runtime qualification is
active.

## 4. Run the K13.D multi-GPU/resilience qualification

```bash
./tools/run-k13d-qemu-gpu.sh
```

The QEMU topology intentionally contains:

- stdvga/GOP as the frozen K12 recovery scanout;
- one active modern VirtIO-GPU candidate with two advertised outputs;
- one additional modern VirtIO-GPU as a secondary/standby topology candidate.

The second GPU is not automatically granted DMA.

Important K13.D markers:

```text
[RSLN] GPU health/rebind state machine:
[HOTG] PCIe GPU hotplug policy self-test:
[MOUT] multi-scanout policy self-test:
[MGP2] multi-GPU presentation policy:
[SOAK] presentation stress:
[DLOS] controlled device-loss fence:
[REBD] transport rearm verified:
[GRDY] K13.D resilience/multi-GPU ready:
[USER] [displayd] K13.D display/compositor service online; resilience enabled
[URCV] DISPLAYD recovery:
[USER] [displayd] K13.D capability-mediated GPU recovery verified
[KERN] K13.D alive:
[QUAL] K13.D robustness runtime reached intentional post-userspace halt
[HALT] BSP halted intentionally
```

## 5. Re-check a saved log

```bash
./tools/check-k13d-serial-log.sh build/k13d-serial.log
```

Target:

```text
Titanweave K13.D robustness/runtime qualification PASSED.
```

## Qualification meaning

Passing K13.D proves the generic/QEMU GPU stack survives a bounded stress run,
can fence/rearm presentation while retaining GOP fallback, handles multi-adapter
and output topology policy, and exposes capability-mediated recovery to
DISPLAYD. It is not a production qualification of native physical GPUs.
