# K15.3 — ForgeAudio Audio DMA Transport

K15.3 is gate 3 of the locked 16-gate ForgeAudio stone contract. It builds on frozen K15.2 and implements the backend-neutral DMA transport that K15.4 HDA will drive. The gate does **not** invent an audio device in QEMU and does not claim a synthetic IRQ or fake DMA completion as hardware evidence.

## Implemented

This gate provides the complete backend-neutral cyclic DMA transport, including IOMMU admission, explicit buffer ownership, and XRUN detection. No audio device is fabricated for qualification.

### Real bounded DMA memory

`kernel/weavecore/src/forgeaudio_dma.rs` allocates physically contiguous pages from Titanweave's reclaiming `FrameAllocator`, maps them through the supervisor-only cacheable kernel DMA aperture with `paging::map_kernel_dma`, wipes the memory on teardown, unmaps the kernel aperture, and returns the physical pages. The transport ring is capped at 1 MiB and at 32 periods.

### Cyclic period ring

Each transport owns a fixed descriptor array with exact physical/virtual offsets, byte length, frame capacity, committed frame count, sequence and ownership state. Playback and capture use distinct legal ownership transitions instead of sharing an ambiguous flag.

Playback:

`CpuWritable -> QueuedToDevice -> DeviceOwned -> CpuWritable`

Capture:

`DeviceReady -> DeviceOwned -> CpuReadable -> DeviceReady`

The transport maintains the next device period, one bounded in-flight period, cumulative frame position, byte position, completed-period count and ring wrap count.

### Production hardware entry points

`backend_acquire_next` and `backend_complete_period` reject all access until `arm_hardware` succeeds. Hardware arming requires a `DmaIsolationLease` that names a nonzero requester/domain/generation, exact physical coverage, an IOVA aperture large enough for the complete ring, and direction-correct permissions. Playback requires device-read permission; capture requires device-write permission.

The backend receives the translated device address (IOVA), not a raw physical address, once the transport is armed. K15.4 HDA is responsible for creating the actual requester-specific translated mapping before constructing this lease.

### IOMMU fail-closed behavior

K15.3 consumes the already-qualified K14.B hardware-translation state. The QEMU qualification still uses the real EDU endpoint only as K14.B's translated-DMA proof; it is **not** registered as audio hardware. K15.3 explicitly verifies that an untranslated/incomplete lease cannot arm an audio transport.

No HDA requester exists until K15.4, so K15.3 reports audio hardware arming as deferred rather than fabricating a mapping.

### XRUN detection

Playback acquisition of a period that has not been queued increments a bounded underrun counter and returns an error without advancing the ring. Capture acquisition of a period that has not been released by the CPU increments an overrun counter and returns an error. Invalid/mismatched completions are rejected.

### Runtime qualification

The boot-time self-test allocates real DMA-capable playback and capture rings, writes and verifies deterministic data through the kernel DMA mapping, exercises cyclic ownership/completion accounting across multiple wraps, validates cumulative frame/byte position, forces exactly one playback underrun and one capture overrun, verifies untranslated hardware arming is rejected, then wipes/unmaps/releases both rings.

The self-test directly drives the transport-core state machine and labels that evidence `completion_source=transport_core_selftest`; production backend APIs remain hardware-arm protected. This is deliberate separation between transport correctness and the HDA hardware/IRQ evidence reserved for K15.4.

A passing kernel emits:

```text
[K15OK] K15.3 ForgeAudio audio DMA transport qualified: cyclic=true period_completion=true position=true ownership=true iommu_fail_closed=true ... xrun=true hardware_audio=false fake_dma=false
[K15DR] ForgeAudio DMA ready: version=1 ... qemu_hda_deferred=true
```

K15.4 remains blocked until K15.3 passes Fedora/QEMU runtime qualification.
