# Titanweave K12 Fedora Tester Guide

K12 forks from the frozen K11 archive. Do not overwrite the frozen K11 tree.

## 1. Host readiness

```bash
cd ~/Downloads/titanweave-kernel-k12-integrated
./tools/build-doctor.sh
```

If packages/toolchains are missing:

```bash
./tools/setup-fedora.sh
source "$HOME/.cargo/env"
```

## 2. Source validation

```bash
./tools/validate-source.sh
```

Expected final line:

```text
Titanweave K1-K12 integrated source validation passed (K12 runtime qualification still required).
```

## 3. Rust build

```bash
PROFILE=debug ./tools/build.sh
```

Do not run `cargo fix` blindly on WeaveCore. Review low-level kernel changes
manually.

## 4. Visible QEMU graphics qualification

```bash
./tools/run-k12-qemu-display.sh
```

A QEMU window should open. After UEFI handoff, Titanweave should visibly render
the K12 Workplace Shell reference preview: blue-steel desktop, top system bar,
left navigator rail, OS/2-style utility windows, and bottom taskbar.

The serial console should include:

```text
[GFX ] K12 GOP scanout online:
[COMP] surface/damage self-test:
[INPT] focus/capture self-test:
[FGFX] ForgeGraphics ABI v1 backend contract passed
[WPS ] Workplace Shell reference preview rendered:
[USER] [displayd] K12 display/compositor service online
[KERN] K12 alive:
[QUAL] K12 runtime reached intentional post-userspace halt
[HALT] BSP halted intentionally
```

The kernel intentionally halts at the end. Ctrl+C is safe.

## 5. Check qualification

```bash
./tools/check-k12-serial-log.sh build/k12-serial.log
```

Expected final line:

```text
Titanweave K12 display/runtime milestone qualification PASSED.
```

## Useful overrides

Headless serial run:

```bash
K12_DISPLAY=none ./tools/run-k12-qemu-display.sh
```

SDL instead of GTK:

```bash
K12_DISPLAY=sdl ./tools/run-k12-qemu-display.sh
```

Disable VT-d if QEMU's emulated IOMMU blocks debugging:

```bash
K12_IOMMU=0 ./tools/run-k12-qemu-display.sh
```

Use more memory/CPUs:

```bash
K12_MEMORY=4096M K12_CPUS=8 ./tools/run-k12-qemu-display.sh
```
