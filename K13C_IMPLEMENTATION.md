# K13.C Implementation Record — Buffered Compositor Presentation

K13.C forks from the frozen, QEMU-qualified K13.B archive. K13.B remains the
known-good modern VirtIO-GPU command-transport checkpoint.

## Purpose

K13.B proved that Titanweave can own a GPU through ForgeBus, negotiate modern
VirtIO PCI, establish queues/DMA backing, create a resource, bind a scanout, and
transfer/flush a frame. K13.C turns that one-shot bootstrap proof into a reusable
presentation path suitable for DISPLAYD.

This slice is intentionally a **GPU-presented software compositor path**. Pixel
composition is still performed by Titanweave code into shared backing memory;
the GPU transport owns scanout/resource presentation. Native GPU blit/raster
engines are later K13 work and are not falsely claimed here.

## Live transport ownership

K13.C preserves the initialized K13.B transport in a kernel `SpinLock` instead
of discarding its software queue state after bootstrap. K13.C presentation then
reuses:

- the same ForgeBus-owned `DeviceId`;
- the same bounded DMA domain;
- the same modern VirtIO capabilities;
- the same control/cursor queues;
- the same bus-master authorization;
- the same timeout/reset behavior.

K13.C does not claim the PCI device a second time and does not create a second
unowned DMA path.

## Buffered presentation

Three 2D resources are maintained as scanout-capable presentation buffers.
Resource 1 is the K13.B bootstrap buffer. Resources 2 and 3 are allocated from
the already-owned ForgeBus DMA domain and receive their own backing memory.

Each buffer is fully initialized once. Subsequent frames rotate through the
three resources and upload only the qualified damage rectangle before switching
scanout.

## Damage handling

`gpu_present::DamageRect` provides bounded clipping and checked conversion from
2D pixel coordinates to the byte offset required by
`VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D`.

The K13.C qualification sequence performs multiple partial updates at distinct
screen locations and verifies that the transport accepts the partial transfer +
flush sequence.

## Fence verification

K13.C sets `VIRTIO_GPU_FLAG_FENCE` on presentation flushes. Each submitted fence
ID must be echoed by the device in the response header before that frame is
considered complete. Fence IDs are monotonic and presentation refuses to exceed
the configured in-flight bound.

The current bootstrap implementation still polls the used ring. Therefore K13.C
qualifies **fence-verified completion**, not asynchronous MSI-driven completion.
Moving continuous presentation to interrupt-driven completion remains a later
hardening/performance step.

## Frame pacing policy

The backend-neutral presentation policy defines:

- three present buffers;
- a maximum of two in-flight frames;
- a 60,000 mHz default pacing target;
- checked nanosecond frame-period calculation;
- a three-stall fallback threshold.

The current QEMU qualification is fence-gated rather than wall-clock/vblank
scheduled. Precise vblank/VRR pacing requires the later display interrupt path.

## DISPLAYD mediation

K13.C adds syscall 9 (`SYS_GPU_PRESENT`). The syscall is rejected unless the caller supplies the graphics-present capability handle installed only for the Display service. DISPLAYD requests a compositor test frame;
the kernel mediates that request into the already-owned backend and returns the
completed fence ID.

This proves that userspace desktop policy can request presentation without
receiving direct MMIO, queue, or DMA ownership.

Direct DISPLAYD-owned shared surface buffers are still a later step; K13.C keeps
GPU DMA allocations kernel/ForgeBus-owned while establishing the capability
boundary first.

## Fallback behavior

The K12 GOP framebuffer is never removed. Presentation policy keeps the fallback
armed, and a failed DISPLAYD present is reported as a fallback event rather than
granting broader GPU access. The K13.C qualification gate requires the live
accelerated presentation path to succeed while also verifying that GOP fallback
remains available.

## Not claimed

K13.C does not yet claim:

- native AMD/Intel/NVIDIA GPU command submission;
- GPU shader/raster composition;
- asynchronous MSI/MSI-X presentation completion;
- hardware vblank/VRR timing;
- complete hardware VT-d/AMD-Vi translated GPU DMA;
- arbitrary userspace-provided scanout buffers;
- multi-monitor atomic presentation.
