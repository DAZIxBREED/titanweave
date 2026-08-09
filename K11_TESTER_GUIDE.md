# TITAN//WEAVE K11 tester guide

This tree is the real K1-K11 source with the K11 integration repairs applied.
The purpose of this package is to let the maintainer perform the compile, UEFI,
QEMU and physical-hardware qualification that could not be executed in the
artifact environment.

## 1. Fedora host setup

```bash
chmod +x tools/*.sh
./tools/setup-fedora.sh
```

Log out/in or run `source "$HOME/.cargo/env"` if `cargo` is not immediately in PATH.

## 2. Source gates

```bash
./tools/validate-source.sh
```

## 3. Compile the complete image

```bash
PROFILE=debug ./tools/build.sh
```

For the optimized build:

```bash
PROFILE=release ./tools/build.sh
```

## 4. Baseline boot

```bash
./tools/run-qemu.sh
```

## 5. K11 hardware-focused virtual boot

This adds a QEMU NVMe controller, qemu-xHCI, USB keyboard/tablet and an Intel
VT-d ACPI/IOMMU device in addition to the retained VirtIO system disk.

```bash
./tools/run-k11-qemu-hardware.sh
```

Disable virtual VT-d if your QEMU build rejects it:

```bash
K11_IOMMU=0 ./tools/run-k11-qemu-hardware.sh
```

The serial log is saved to `build/k11-serial.log`.

## 6. Check and package the result

```bash
./tools/check-serial-log.sh build/k11-serial.log
./tools/collect-k11-report.sh
```

Send back `build/K11_TEST_REPORT.txt` and, on compilation failure, the complete
Cargo error output.

## 7. Physical machine test

Do **not** overwrite the machine's existing EFI System Partition or production
disks. Use a dedicated USB drive / spare disk for initial bare-metal testing.
The K11 QEMU pass should be completed first.

For first bare-metal bring-up, capture serial output if the motherboard exposes
a usable serial interface. Confirm in this order:

1. UEFI reaches `BOOTX64.EFI`.
2. WeaveCore v11 handoff succeeds.
3. ACPI/MADT and SMP initialize.
4. ForgeBus enumerates PCI/PCIe functions.
5. IOMMU reports AMD-Vi or Intel VT-d when available.
6. NVMe/xHCI devices enumerate without a panic.
7. Native K11 services reach the stable-userspace handoff.
8. The kernel performs its intentional qualification halt.

A panic or unexpected `[FAIL]` line is useful test data; preserve the entire log.
