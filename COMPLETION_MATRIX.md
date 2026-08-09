# Titanweave Completion Matrix

## Project milestone status

| Milestone | Scope | Status |
|---|---|---|
| K1-K10 | Early kernel, platform, storage/trust foundations | **qualified/frozen where completed** |
| K11 | ForgeBus foundation and runtime closure | **QUALIFIED / FROZEN ✅** |
| K12 | Graphics/display foundation | **QUALIFIED / FROZEN ✅** |
| K13.A-D | Generic acceleration, VirtIO-GPU, compositor presentation, resilience/multi-GPU groundwork | **QUALIFIED / FROZEN ✅** |
| K14.A-B | Native-GPU prerequisites and translated-DMA foundations | **QUALIFIED / FROZEN ✅** |
| K14.C1-C25 | Progressive Radeon ownership, AMD-Vi, discovery, safe MMIO/read/write qualification | **QUALIFIED / FROZEN ✅** |
| K14.C26 | Safe hardware/MMIO foundation | **QUALIFIED / FROZEN ✅** |
| K14.C27 | Complete Radeon driver core | **QUALIFIED / FROZEN ✅** |
| K14.C28 | Memory + firmware + recovery | **QUALIFIED / FROZEN ✅** |
| K14.C29 | Rings + queues + fences + DMA | **QUALIFIED / FROZEN ✅** |
| K14.C30 | Complete basic display engine | **QUALIFIED / FROZEN ✅** |
| K14.C31 | Graphics + compute execution | **QUALIFIED / FROZEN ✅** |
| K14.C32 | Production/stability + final K14 | **QUALIFIED / FROZEN ✅** |
| **K14** | **Native Radeon foundation** | **COMPLETE / FROZEN ✅** |
| **K15** | **ForgeAudio** | **NEXT 🔊** |

---

## K14 closure detail

### K14.C26 — Safe hardware/MMIO foundation

Qualified/frozen. Established the source-reviewed GFX12 MMIO foundation, exact safe target/allowlist behavior, bounded proof paths, and fail-closed authority boundaries inherited by the remaining Radeon milestones.

### K14.C27 — Complete Radeon driver core

Qualified/frozen. Operational driver lifecycle, ForgeBus ownership, resource/topology state, reviewed MMIO service, error/reset coordination, software interrupt route/handler accounting, and userspace status interface.

### K14.C28 — Memory + firmware + recovery

Qualified/frozen. Operational GTT allocation/map/reclaim, VRAM reservations, GPU virtual-address ownership, firmware parse/CRC/SHA staging, watchdog behavior, and software recovery/resource reclamation.

### K14.C29 — Rings + queues + fences + DMA

Qualified/frozen. Operational GTT-backed ring, FIFO submission queue lifecycle, timeline fence, typed SDMA packet model, owned-memory DMA/reference execution, and source-reviewed GFX12 SDMA queue authority plan.

### K14.C30 — Complete basic display engine

Qualified/frozen. EDID/mode engine, connector/CRTC/plane ownership, double-buffered GTT scanout, live GOP framebuffer presentation/page flips, atomic rollback, hotplug bookkeeping, and DCN401 resource modeling.

### K14.C31 — Graphics + compute execution

Qualified/frozen. Owned shader/resource upload, typed command encoding, separate graphics and compute queues, verified vector-add dispatch, verified triangle draw/live framebuffer presentation, timeline-fence retirement, shader cache/precache, and capability reporting.

### K14.C32 — Production/stability + final K14

**Qualified/frozen on 2026-08-09.** Final Fedora/QEMU qualification passed:

- queue wrap/stability stress,
- memory pressure/reclaim,
- stuck-queue detection and recovery,
- interrupt/recovery stress,
- graphics+compute coexistence,
- display+compute coexistence,
- repeated scanout stability,
- multi-display framework checks,
- multi-GPU PCI inventory groundwork,
- telemetry/performance diagnostics,
- software power policy,
- shader-precache freeze,
- syscall-43 userspace GPU ABI/capability freeze,
- `displayd` userspace status,
- stable userspace handoff,
- final `[QUAL]`, `[K14DONE]`, `[K15NEXT]`, and intentional `[HALT]` markers.

Final result:

```text
[K14DONE] Titanweave native Radeon driver foundation operational
```

Raw QEMU exit status after intentional halt: `0`.

Physical Radeon silicon stress remains a separate bare-metal evidence track and is not falsely inherited from QEMU.

---

## Current development target

# K15 — ForgeAudio

K14 is closed. K15 begins Titanweave's native low-latency audio foundation.

See `README.md`, `PROJECT_VISION.md`, `K14_STATUS.md`, and `BUILD_STATUS.md` for project direction and qualification details.