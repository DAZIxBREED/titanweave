# Titanweave K14.B Fedora/QEMU Tester Guide

K14.B validates real Intel VT-d translation using a deterministic QEMU EDU DMA
endpoint while retaining the frozen K13.D/K14.A graphics path.

## Build

```bash
cd ~/Downloads/titanweave-kernel-k14b-integrated
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
```

Do not run `cargo fix` on the kernel warning set during qualification.

## Run the translated-DMA qualification

```bash
./tools/run-k14b-qemu-iommu.sh
```

The QEMU profile contains:

- Q35 + `intel-iommu,caching-mode=on`
- the frozen K13 dual VirtIO-GPU topology
- stdvga/GOP fallback
- QEMU EDU PCI DMA test endpoint (`1234:11e8`)
- NVMe, xHCI, keyboard and tablet regression devices

## Expected new K14.B markers

```text
[IOM2] K14.B translated-DMA foundation: ...
[IOMH] Intel VT-d hardware engine: ...
[IOVA] translated DMA map: ...
[DMAT] EDU translated DMA round-trip verified: ...
[IOPF] unmapped DMA denied: ... destination_unchanged=true ...
[INVL] VT-d context/IOTLB invalidation verified: ...
[REVK] translated DMA test domain revoked: ... bus_master=false
[IOMR] hardware translation qualification: backend=IntelVtd translated=true ...
[IOMF] K14.B translated DMA qualification ready: ... translated=true ...
[IOMQ] native DMA admission: iommu=HardwareTranslated hardware_translation=true device_domain_bound=false bus_master_authorized=false
```

DISPLAYD should additionally print:

```text
[displayd] K14.B hardware-translated DMA engine qualified
```

The expected finish is:

```text
[KERN] K14.B alive: ...
[QUAL] K14.B translated-DMA runtime reached intentional post-userspace halt
[HALT] BSP halted intentionally
Titanweave K14.B translated-DMA runtime qualification PASSED.
```

## Important interpretation

`hardware_translation=true` means the VT-d engine successfully translated and
blocked real DMA in the qualification window. `device_domain_bound=false` is
also expected: K14.B does not yet attach a physical native GPU. Therefore
`bus_master_authorized=false` for native GPU activation is the correct result.

## If it fails

Paste the complete terminal output. The most useful failure markers are
`[IOMH]`, `[IOVA]`, `[DMAT]`, `[IOPF]`, `[INVL]` and the first `[FAIL]` line.
