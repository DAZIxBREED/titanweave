# Titanweave OS

**A ground-up 64-bit operating system focused on performance, gaming, creators, low-latency media, hardware freedom, and practical compatibility.**

Titanweave is not a Linux distribution, a Unix derivative, or a Windows modification. It is an independent operating-system project built around **WeaveCore**, with its own kernel, driver model, graphics stack, storage direction, userspace, and long-term compatibility architecture.

The project takes inspiration from the directness and modularity of systems such as OS/2 while targeting a modern desktop for gamers, artists, DJs, developers, VR users, and high-performance workstation owners.

## What Titanweave is trying to do

Titanweave is designed around capabilities that are difficult to bolt onto a mature operating system after the fact:

- **Performance by design** — explicit scheduling, ownership, DMA, memory, and latency behavior instead of hidden background overhead.
- **Gaming as a first-class workload** — shader precaching, predictable scheduling, fast asset access, low-overhead graphics paths, and practical Windows-software compatibility through clean-room translation.
- **Multi-GPU from the architecture upward** — multiple GPUs are intended to become schedulable resources, including mixed-vendor systems, rather than a late compatibility feature.
- **Creator and DJ workflows as first-class workloads** — ForgeAudio is the native low-latency audio foundation for professional routing, DJ hardware, media, capture, streaming, and production workflows.
- **VR as a first-class platform** — display, graphics, audio, timing, input, and scheduling are intended to support VR directly.
- **Easy driver development** — ForgeBus and capability-scoped hardware interfaces are intended to make custom drivers understandable, testable, and safer to build.
- **Native accelerator direction** — long-term support for AMD HIP/ROCm-class, NVIDIA CUDA/OptiX-class, and Intel/Embree-class workloads while keeping Titanweave interfaces vendor-neutral where practical.
- **System RAM as a performance tier** — abundant DDR4/DDR5 can be assigned to storage caching and other high-throughput workloads rather than sitting idle.
- **Modern storage without sacrificing boot compatibility** — FAT32 is isolated to the EFI System Partition where firmware requires it; internal storage is not forced to use firmware-era formats.

The ambition is intentionally larger than the currently completed implementation. Titanweave documents what is **implemented**, what is **runtime-qualified**, what is **frozen**, and what remains **future work** instead of presenting roadmap goals as finished features.

See [`PROJECT_VISION.md`](PROJECT_VISION.md) for the long-form direction and [`CURRENT_STATUS.md`](CURRENT_STATUS.md) for the current development gate.

---

## Current status

### K14 — Native Radeon foundation: **COMPLETE / QUALIFIED / FROZEN ✅**

K14.C32 passed Fedora/QEMU production/stability qualification on 2026-08-09 through the final post-userspace completion markers, including:

```text
[C32OK] K14.C32 production/stability + final K14
[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt
[K14DONE] Titanweave native Radeon driver foundation operational
[K15NEXT] K15 ForgeAudio is the next locked Titanweave milestone
[HALT] BSP halted intentionally
```

QEMU stopped after Titanweave's intentional halt with raw exit status `0`. Physical Radeon silicon stress remains a separately evidenced bare-metal qualification and is not falsely inferred from QEMU.

### K15 — ForgeAudio: **IN PROGRESS 🔊**

ForgeAudio is governed by the locked 16-gate contract in [`K15_STONE_CONTRACT.md`](K15_STONE_CONTRACT.md).

| Gate | Scope | Status |
|---|---|---|
| K15.1 | Real-Time Audio Execution Foundation | **FROZEN ✅** |
| K15.2 | ForgeAudio Kernel ABI | **FROZEN ✅** |
| K15.3 | Audio DMA Transport | **FROZEN ✅** |
| K15.4 | Real HDA Hardware Backend | **FROZEN ✅** |
| K15.5 | PCM Format Engine | **FROZEN ✅** |
| K15.6 | ForgeAudioD | **FROZEN ✅** |
| K15.7–K15.16 | ForgeAudio transport, graph, sync, resampling, routing, recovery and final qualification | **LOCKED** |

K15.4 and K15.5 are qualified/frozen. K15.5 passed the canonical PCM format/rate/channel engine and real HDA endpoint binding. K15.6 now adds ForgeAudioD as a real persistent userspace service that owns the HDA device, playback/capture stream objects, bounded buffers, clock/event/fence control objects, two-route control metadata, telemetry and recovery/rebuild state. K15.7 remains locked until K15.6 passes.

---

## Architecture direction

### Kernel and hardware

- **WeaveCore** 64-bit kernel
- capability-scoped hardware access
- ForgeBus device and driver ownership
- bounded, fail-closed hardware bring-up
- explicit DMA/IOMMU ownership
- stable userspace hardware ABIs after qualification
- multi-GPU inventory and future scheduling

### Graphics

K14 established the native Radeon foundation through memory/firmware/recovery, queues/fences/DMA, display, graphics/compute execution, and production/stability gates. QEMU reference execution and physical-silicon evidence remain explicitly distinguished.

### Audio

K15 ForgeAudio is building the native audio stack in sixteen frozen gates: real-time execution, kernel ABI, DMA transport, HDA hardware, PCM formats, ForgeAudioD, lock-free transport, graph execution, sample-accurate switching, clocks/synchronization, resampling, full duplex, routing/mixing/monitoring, latency/XRUN handling, fault recovery/hotplug, and final production qualification.

### Storage

- GPT disks
- FAT32 EFI System Partition only where firmware requires it
- TitanBoot on the ESP
- NTFS as the early-development default for non-boot internal volumes
- exFAT for removable/shared media
- TitanFS as the long-term native filesystem
- safe automatic discovery/mount policy
- native archive/package direction based around 7-Zip support

### Compatibility and compute

Long-term Titanweave work includes Windows application/API translation, modern graphics translation, native accelerator integration, mixed-vendor compute, shader precaching, and workload-aware memory/caching policies. These are roadmap goals and are not claimed as completed merely because their interfaces are anticipated by the architecture.

---

## Development rule

**No fake success. No required-path stubs.**

A milestone is not frozen because source exists. It must pass its source gates and the appropriate runtime qualification. QEMU/emulated hardware evidence is labeled as such, and physical-silicon claims require physical evidence.

For the exact active state, see [`CURRENT_STATUS.md`](CURRENT_STATUS.md).
