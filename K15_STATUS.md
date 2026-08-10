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

Consumes frozen K15.3 and implements real PCI HDA discovery/ForgeBus ownership, BAR0 MMIO reset, CORB/RIRB command DMA, codec/function-group/widget discovery, HDA BDL/stream-descriptor programming, exact-requester translated VT-d data DMA, PCI MSI through Titanweave's interrupt router, real playback/capture interrupt completion, capture-memory mutation verification, and real HDA device/playback/capture endpoint registration. Fedora/QEMU runtime qualification passed with peer DMA/GPU coexistence preserved, `fake_hw=false`, and `physical_silicon=false`.

## K15.5 — PCM Format Engine

**QUALIFIED / FROZEN.**

Builds on frozen K15.4. Implements allocation-free canonical PCM representation for S16, S24-in-32, S32 and F32; bounded interleaved/planar conversion; explicit channel-position maps and non-mixing channel remap/zero-fill; canonical HDA rate/width capability parsing; exact and nearest supported-rate negotiation; HDA PCM stream-format encode/decode; K15.3-compatible period/ring geometry; and binding against the real HDA playback/capture endpoints registered by K15.4. Fedora/QEMU runtime qualification passed canonical PCM formats, rate negotiation, interleaved/planar conversion, channel mapping, HDA encode/decode, bounded period geometry and real HDA endpoint binding. K15.5 is frozen.


## K15.6 — ForgeAudioD

**QUALIFIED / FROZEN.**

Builds directly on frozen K15.5. Adds a real persistent `forgeaudiod` userspace service, singleton kernel registration, real HDA device/endpoint ownership, prepared playback/capture streams, bounded server buffers, clock/event/fence ownership, two-route control metadata with generation tracking, userspace recovery/rebuild proof, kernel-validated ownership publication, and a post-yield persistent heartbeat. K15.7 remains locked until K15.6 runtime qualification passes.
