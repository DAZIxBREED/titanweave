# Titanweave K13.B Fedora/QEMU Test Guide

This tree must be tested separately from the frozen K13.A archive.

## 1. Enter the K13.B tree

```bash
cd ~/Downloads/titanweave-kernel-k13b-integrated
```

## 2. Run source/regression validation

```bash
./tools/validate-source.sh
```

Expected final line:

```text
Titanweave K1-K13.B integrated source validation passed (K13.B runtime qualification still required).
```

## 3. Build the complete image

```bash
PROFILE=debug ./tools/build.sh
```

Do not run `cargo fix` on the kernel warning set during qualification.

## 4. Run the K13.B GPU transport qualification

```bash
./tools/run-k13b-qemu-gpu.sh
```

The QEMU configuration keeps two graphics paths:

- `stdvga` — inherited K12 UEFI GOP/recovery scanout;
- `virtio-gpu-pci` — K13.B modern transport candidate.

The VirtIO device intentionally uses `iommu_platform=off` in this milestone
because Titanweave has not yet implemented real VT-d/AMD-Vi translation tables.
ForgeBus still bounds and tracks all transport DMA allocations.

## 5. Required new K13.B markers

The serial log must include:

```text
[VPCI] modern capabilities + VERSION_1 negotiated:
[VQ  ] controlq=... cursorq=... polled bootstrap completion online
[VDMA] ForgeBus bounded DMA ownership online:
[SCAN] VirtIO-GPU resource 1 scanout=... transfer+flush verified
[GACC] ForgeGraphics acceleration ABI v1 transport passed transport_ready=true backend=virtio-gpu-modern
[GPUT] K13.B VirtIO-GPU transport ready:
[USER] [displayd] K13.B display/compositor service online
[USER] [displayd] K13.B VirtIO-GPU command transport online
[KERN] K13.B alive:
[QUAL] K13.B transport runtime reached intentional post-userspace halt
[HALT] BSP halted intentionally
```

The runner automatically calls:

```bash
./tools/check-k13b-serial-log.sh build/k13b-serial.log
```

A successful run ends with:

```text
Titanweave K13.B transport/runtime qualification PASSED.
```

## If transport setup fails

Paste the full serial output beginning at `[GPU ] K13 topology:` through the
`[FAIL] K13.B VirtIO-GPU transport failed:` line. Capability layout, feature
negotiation, queue setup, and command-response failures are intentionally kept
separate so the failing stage can be identified quickly.
