# Titanweave OS

**A ground-up 64-bit operating system focused on performance, gaming, creators, low-latency media, hardware freedom, and practical compatibility.**

Titanweave is not a Linux distribution, a Unix derivative, or a Windows modification. It is an independent operating-system project built around **WeaveCore**, with its own kernel, driver model, graphics stack, storage direction, userspace, and long-term compatibility architecture.

The project is inspired by the directness and modularity of systems such as OS/2 while aiming at a modern desktop that is practical for gamers, artists, DJs, developers, VR users, and workstation owners.

## The ambition

Titanweave is being designed around a few ideas that are difficult to bolt onto an existing operating system after the fact:

- **Performance by design.** Keep unnecessary background work out of critical paths and make latency, scheduling, memory ownership, and hardware access explicit.
- **Gaming as a first-class workload.** Build toward shader precaching, fast asset access, predictable scheduling, low-overhead graphics paths, and practical Windows-software compatibility through translation rather than copying Windows.
- **Multi-GPU from the architecture upward.** Treat multiple GPUs as schedulable resources instead of an afterthought, with long-term support for mixed-vendor systems and workload-aware execution.
- **Creator and DJ workflows as first-class workloads.** K15 begins **ForgeAudio**, Titanweave's native low-latency audio foundation. The broader goal includes pro-audio routing, high-rate audio, DJ hardware, media codecs, streaming, and creator workloads without layers of unnecessary latency.
- **VR as a first-class platform.** Display, input, graphics, audio, timing, and scheduling are intended to support VR rather than treating it as an external compatibility case.
- **Easy driver development.** ForgeBus and the driver architecture are intended to make custom hardware support understandable, capability-scoped, testable, and safer to implement.
- **Native accelerator support.** The long-term compute direction includes AMD HIP/ROCm-style capability support, NVIDIA CUDA/OptiX-class workloads, and Intel/Embree-class acceleration while keeping Titanweave's own interfaces vendor-neutral where possible.
- **Memory as a performance tier.** Titanweave's long-term design includes assignable DDR4/DDR5 caching for storage and other workloads, including systems where abundant system RAM can reduce I/O pressure or supplement accelerator workflows.
- **Modern storage without sacrificing boot compatibility.** Firmware compatibility stays isolated to the EFI System Partition instead of dictating the rest of the storage stack.

The ambition is intentionally larger than the currently completed implementation. Titanweave documents what is **operational**, what is **qualified**, and what remains **future work** rather than treating roadmap goals as finished features.

See [`PROJECT_VISION.md`](PROJECT_VISION.md) for the long-form design direction.

---

# Current status

## K14 — Native Radeon foundation: **COMPLETE / FROZEN ✅**

On **2026-08-09**, Titanweave K14.C32 passed its Fedora/QEMU production/stability qualification through the final post-userspace markers:

```text
[C32OK] K14.C32 production/stability + final K14
[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt
[K14DONE] Titanweave native Radeon driver foundation operational
[K15NEXT] K15 ForgeAudio is the next locked Titanweave milestone
[HALT] BSP halted intentionally
```

The final QEMU run exited with raw status `0` after Titanweave's intentional halt.

### Frozen Radeon closure

| Milestone | Scope | Status |
|---|---|---|
| K14.C26 | Safe hardware/MMIO foundation | **FROZEN ✅** |
| K14.C27 | Complete Radeon driver core | **FROZEN ✅** |
| K14.C28 | Memory + firmware + recovery | **FROZEN ✅** |
| K14.C29 | Rings + queues + fences + DMA | **FROZEN ✅** |
| K14.C30 | Complete basic display engine | **FROZEN ✅** |
| K14.C31 | Graphics + compute execution | **FROZEN ✅** |
| K14.C32 | Production/stability + final K14 | **FROZEN ✅** |
| **K14** | **Native Radeon foundation** | **COMPLETE ✅** |
| **K15** | **ForgeAudio** | **NEXT 🔊** |

K14.C32 qualified queue/memory stress, recovery and interrupt stress, graphics+compute and display+compute coexistence, repeated scanout, telemetry/diagnostics, power policy, frozen GPU ABI/capabilities, shader precache integration, multi-GPU inventory groundwork, stable userspace handoff, and the final K14 completion path.

**Important qualification boundary:** QEMU proves the Titanweave software/reference execution and safety paths. It does not pretend to prove physical Radeon silicon stress. Physical Radeon execution and stress evidence remain separate bare-metal qualifications where required.

See [`K14_STATUS.md`](K14_STATUS.md), [`BUILD_STATUS.md`](BUILD_STATUS.md), and [`COMPLETION_MATRIX.md`](COMPLETION_MATRIX.md).

---

# Architecture direction

## Kernel and hardware

- **WeaveCore** 64-bit kernel
- capability-scoped hardware access
- ForgeBus device/driver ownership model
- bounded, fail-closed bring-up policies
- explicit DMA/IOMMU ownership
- first-class multi-GPU inventory and future scheduling
- stable userspace hardware ABIs only after qualification

## Graphics

Titanweave's graphics work currently includes the generic ForgeGraphics safety baseline plus the completed K14 Radeon foundation. K14 established memory ownership, queues, fences, typed command submission, display ownership, graphics/compute reference execution, shader cache/precache hooks, recovery, diagnostics, and a frozen userspace capability ABI.

Future work can build on that foundation without reopening the frozen K14 contract.

## Audio

**K15 ForgeAudio is next.**

ForgeAudio is intended to become Titanweave's native low-latency audio subsystem, with architecture suitable for gaming, VR, professional audio, DJ hardware, routing, capture, streaming, and creator workloads.

## Storage

The current storage direction is:

1. **GPT** disk layout.
2. **FAT32 EFI System Partition** only where firmware requires it.
3. **TitanBoot** on the ESP.
4. During early development, default non-bootable internal volumes use **NTFS**.
5. **exFAT** remains supported primarily for removable/shared media.
6. **TitanFS** is the planned native production filesystem.

Firmware therefore only needs standards-compliant FAT32 support; Titanweave itself owns the rest of the storage policy.

## Compatibility

Titanweave's long-term compatibility goal is to run important Windows software through a clean translation/compatibility architecture while remaining its own operating system. Compatibility is a design target, not an excuse to import Windows internals or make Titanweave dependent on a Windows installation.

---

# Engineering rules

Titanweave milestone work follows several hard rules:

- **No fake-success subsystems.**
- **No `todo!()`, `unimplemented!()`, or placeholder implementation presented as complete.**
- Frozen milestones stay frozen except for qualification metadata or forward-compatible regression fixes that do not change their qualified behavior.
- QEMU safe-defer paths never count as proof of physical hardware execution.
- Hardware authority is opened deliberately and remains fail-closed when prerequisites are absent.
- Source validation is not runtime qualification; both are tracked separately.

---

# Validate and build

From a Titanweave source tree:

```bash
./tools/validate-source.sh
PROFILE=debug ./tools/build.sh
```

K14.C32's final QEMU qualification runner is:

```bash
./tools/run-k14c32-qemu-production-stability-final.sh
```

The project is now advancing to **K15 ForgeAudio**.