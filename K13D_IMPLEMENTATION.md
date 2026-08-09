# K13.D Implementation Record — GPU Resilience and Multi-GPU Robustness

K13.D forks from the user's frozen, QEMU-qualified K13.C archive. K13.C remains
an immutable known-good checkpoint.

## Purpose

K13.A established the backend-neutral GPU contracts. K13.B proved a live modern
VirtIO-GPU transport. K13.C turned that transport into a reusable buffered
presentation path. K13.D hardens the stack so a compositor can survive stalls,
device loss, output changes, and the presence of more than one adapter without
turning GPU failure into a kernel-wide failure.

## Health and recovery state machine

`gpu_resilience.rs` adds a bounded adapter-health state machine with:

- Healthy, Degraded, Resetting, Rebinding, Offline, and Quarantined states;
- generation counters for stale-event rejection;
- a three-stall recovery threshold;
- primary/standby presentation selection;
- surprise-removal failover;
- explicit firmware-fallback readiness.

The deterministic self-test drives a primary adapter through stall detection,
reset/rebind states, successful recovery, then surprise removal and standby
promotion.

## Live K13.C transport resilience qualification

K13.D does not create a second unowned transport. The already-qualified K13.B
VirtIO-GPU device, DMA domain, queues, and K13.C presentation buffers remain the
single active transport.

Runtime qualification performs:

1. 64 live dirty-region presentations with strictly monotonic frame/fence IDs;
2. controlled presentation suspension while the K12 GOP fallback remains armed;
3. proof that presentation requests are rejected while the accelerated path is
   fenced;
4. validation that the owned transport remains bus-master-enabled and
   `DRIVER_OK`;
5. rearming of the existing transport and successful fence-verified presentation.

This is a **controlled transport rearm test**, not a claim of physical PCI FLR,
slot power cycling, or native-vendor engine reset. Those require backend-specific
reset implementations and real hardware qualification.

## Multi-GPU policy

The K13.D QEMU runner exposes two modern VirtIO-GPU PCI functions in addition to
the K12 stdvga/GOP recovery display. Only the first VirtIO-GPU is claimed for
DMA/command transport. The second remains a topology/standby candidate.

This proves that:

- discovery handles multiple adapters;
- presentation ownership is explicit rather than "first GPU owns everything";
- transfer policy can select same-adapter, peer-to-peer, shared-memory, or CPU
  staging routes;
- K13.D does not grant DMA merely because a secondary GPU is present.

Native cross-adapter copy engines are not claimed in K13.D.

## Multi-scanout and hot-plug policy

K13.D adds a bounded four-output topology policy and exercises:

- output connect/disconnect generations;
- primary-output promotion;
- PCIe hot-plug debounce/generation handling;
- surprise-removal state transitions.

The QEMU active VirtIO device advertises up to two outputs, but the current
qualification does not falsely claim a fully interactive two-monitor desktop or
atomic multi-head modeset implementation.

## DISPLAYD recovery mediation

Syscall 10 (`SYS_GPU_RECOVER`) lets DISPLAYD request a controlled GPU
presentation recovery. It reuses the same DISPLAYD-only graphics capability
already required for `SYS_GPU_PRESENT`.

DISPLAYD never receives:

- PCI configuration access;
- MMIO access;
- virtqueue ownership;
- unrestricted DMA ownership.

The kernel performs the fence/rearm sequence and returns the completed fence ID.

## DMA/IOMMU truth boundary

K13.D preserves K13.B/C's existing safety boundary. ForgeBus tracks all GPU DMA
allocations and can revoke them as a device unit, but complete hardware
VT-d/AMD-Vi translated GPU page tables are still not claimed. QEMU therefore
continues to use `iommu_platform=off` for the VirtIO-GPU milestone devices.

## Not claimed by K13.D

K13.D does not yet claim:

- physical PCI FLR or slot power-cycle recovery;
- native AMD/Intel/NVIDIA register/firmware/queue bring-up;
- native GPU raster/shader composition;
- interrupt-driven MSI/MSI-X fence completion;
- hardware vblank/VRR synchronization;
- fully translated hardware-IOMMU GPU DMA;
- peer-to-peer DMA between physical GPUs;
- complete atomic multi-monitor presentation.

Those are later hardware/backend milestones, not hidden behind K13.D PASS
markers.
