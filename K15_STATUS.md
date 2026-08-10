# Titanweave K15 ForgeAudio Status

K15 is governed by `K15_STONE_CONTRACT.md`: exactly 16 gates, no stubs on required execution paths, and no expansion beyond K15.16.

## K15.1 — Real-Time Audio Execution Foundation

**QUALIFIED / FROZEN.**

Fedora/QEMU qualification completed eight periodic audio jobs with zero deadline misses, zero budget exhaustions and zero guard overruns; priority inheritance and bounded preemption deferral were both observed. K14.C32 remained qualified through intentional HALT.

## K15.2 — ForgeAudio Kernel ABI

**QUALIFIED / FROZEN.**

Implements ABI v1 as a shared kernel/userspace crate, real bounded device/endpoint/stream/buffer/clock/event/fence lifecycle, rights-bearing process handles, syscall 44/45/46 exposure, real bounded buffer storage, monotonic clocks/fences, bounded events and strict stream-state transitions. Fedora/QEMU runtime qualification passed with an honest zero-device QEMU registry, strict illegal-start rejection + recovery, 256-byte/64-frame bounded buffer validation, monotonic clock/event/fence qualification, and no fabricated audio hardware.

## K15.3 — Audio DMA Transport

**QUALIFIED / FROZEN.**

Implements real physically contiguous DMA-ring memory, supervisor kernel-DMA mapping/teardown, bounded cyclic period descriptors, strict playback/capture ownership transitions, cumulative frame/byte position, wrap accounting, production hardware-arm gating behind an exact translated-IOMMU lease, translated IOVA device addresses, and explicit underrun/overrun detection. Fedora/QEMU runtime qualification passed 12 cyclic periods / 3 wraps / 1536 frames, strict ownership, translated-platform fail-closed arm gating, one bounded playback underrun and one capture overrun, with `fake_dma=false` and HDA honestly deferred.

K15.3 is frozen. K15.4 — Real HDA Hardware Backend is now unlocked.


## K15.4 — Real HDA Hardware Backend

**QUALIFIED / FROZEN.**

Consumes frozen K15.3 and implements real PCI HDA discovery/ForgeBus ownership, BAR0 MMIO reset, CORB/RIRB command DMA, codec/function-group/widget discovery, HDA BDL/stream-descriptor programming, exact-requester translated VT-d data DMA, PCI MSI through Titanweave's interrupt router, two real playback and two real capture period completions, capture-memory mutation verification, and real HDA device/playback/capture endpoint registration. Fedora/QEMU runtime qualification passed real CORB/RIRB codec communication, translated HDA DMA, BDL playback/capture, MSI-driven stream completion, capture-memory mutation, ForgeAudio endpoint registration and HDA/GPU coexistence. QEMU records `physical_silicon=false` and `fake_hw=false`. K15.4 is frozen; K15.5 — PCM Format Engine is unlocked.
