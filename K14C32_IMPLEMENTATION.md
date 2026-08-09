# Titanweave K14.C32 — Production/Stability + Final K14 Radeon Foundation

Status: **IMPLEMENTED / SOURCE-VALIDATED / RUNTIME QUALIFICATION PENDING**

Frozen prerequisite: **K14.C31 QUALIFIED / FROZEN**.

K14.C32 is the final locked Radeon milestone before K15 ForgeAudio. It does not add a new rendering feature tier; it hardens and freezes the complete K14 Radeon foundation built through C31.

## Operational C32 gates

- **Queue stress:** separate C31 compute and graphics queues each execute 512 submissions/retirements across repeated ring-style index wrap, for 1,024 total queue operations per side of the submit/retire accounting.
- **Hang detection/recovery:** a deliberately started but unretired queue entry is detected, then the queue is reset and its abandoned-work count is verified.
- **Memory pressure:** twelve 1 MiB GTT objects are allocated, sentinel-written, read back, reclaimed in reverse order, and the Radeon memory accounting must return exactly to its pre-test state. When a real Radeon VRAM aperture exists, four additional 1 MiB VRAM reservations are pressure-tested and reclaimed. QEMU records VRAM pressure as deferred rather than faking it.
- **Interrupt stress:** the actual Radeon driver interrupt handler/accounting path executes 1,024 bounded software dispatches without enabling physical Radeon IH/MSI hardware.
- **Recovery stress:** the Radeon driver lifecycle performs 32 quiesce → fault → coordinated-reset → online cycles with exact fault/reset epoch checks. This is software/lifecycle recovery, not a claimed physical ASIC reset.
- **Display + compute / graphics + compute coexistence:** 64 interleaved rounds use real GTT compute buffers, C31 command buffers, separate compute/graphics queues, C29 timeline fences, writes into the C30 scanout target, and repeated live GOP presentation. The compute output is numerically checked after the final round.
- **Display stability:** 16 additional alternating front/back C30 scanout presents must produce repeated live framebuffer changes.
- **Multi-display framework:** all four C30 connector slots are exercised with unique connector/CRTC/plane ownership plus bounded hotplug journal transitions.
- **Multi-GPU inventory:** up to eight real PCI display-class functions are inventoried, including AMD adapter counts. Peer DMA and cross-GPU execution remain false until separately implemented and qualified.
- **Telemetry/diagnostics:** a bounded 32-entry allocation-free event journal records queue, memory, interrupt, recovery, hang, display, power, error and work counters.
- **Power policy:** a bounded Boot/Active/Idle/Quiesced/Fault state machine is executed. Physical SMU/clock programming remains false.
- **Shader precache:** the C31 cache/precache state is required and frozen into the final capability contract.
- **Userspace GPU ABI freeze:** syscall **43** and the C32 packed status/capability bit layout are frozen at ABI version 1.
- **Bare-metal evidence suite:** a separate physical-Radeon serial-log checker is included and intentionally cannot be satisfied by QEMU fallback evidence.

## Authority boundary

QEMU C32 qualification proves Titanweave's production/stability control plane and reference execution foundation. It does **not** prove physical Radeon CP/GFX execution, physical Radeon IH, physical ASIC reset, SMU clock/power programming, peer DMA, or synchronized cross-GPU execution. Those facts remain explicit false/deferred status fields rather than fake-success claims.

C32 contains zero TODO/unimplemented/placeholder subsystems.

## Final marker

Only after C32 survives userspace handoff does the runtime emit:

`[K14DONE] Titanweave native Radeon driver foundation operational`

The next locked milestone is:

`[K15NEXT] K15 ForgeAudio is the next locked Titanweave milestone`
