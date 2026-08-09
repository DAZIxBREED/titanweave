# Titanweave K14 Status

## Overall status

**K14 Native Radeon Foundation: COMPLETE / QUALIFIED / FROZEN ✅**

Final Fedora/QEMU qualification completed successfully on **2026-08-09** at K14.C32.

The final runtime reached all production/stability gates, stable userspace handoff, the intentional post-userspace halt, and the final completion markers:

```text
[C32OK] K14.C32 production/stability + final K14
[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt
[K14DONE] Titanweave native Radeon driver foundation operational
[K15NEXT] K15 ForgeAudio is the next locked Titanweave milestone
[HALT] BSP halted intentionally
```

QEMU stopped after the intentional Titanweave halt with raw exit status `0`.

Physical Radeon stress remains separately evidenced. QEMU qualification proves Titanweave's software/reference execution, safety, lifecycle, concurrency, recovery, display, ABI, and production-gate behavior; it does not falsely claim physical silicon stress.

---

## Frozen closure milestones

### K14.C26 — Safe hardware/MMIO foundation

**Status: QUALIFIED / FROZEN ✅**

Established the final safe reviewed Radeon MMIO foundation, including exact GFX12 `SCRATCH_REG1` identity, reviewed allowlisting, bounded read proof, and zero new uncontrolled MMIO authority.

### K14.C27 — Complete Radeon driver core

**Status: QUALIFIED / FROZEN ✅**

Established the persistent Radeon driver object/lifecycle, ForgeBus ownership, resource/topology capture, reviewed MMIO service, software interrupt route/handler accounting, reset/error coordination, and userspace status interface.

### K14.C28 — Memory + firmware + recovery

**Status: QUALIFIED / FROZEN ✅**

Established operational GTT allocation/map/reclaim, VRAM reservations, GPU virtual-address ownership, AMD firmware parsing/CRC/SHA staging, watchdog behavior, and software recovery/resource reclamation.

### K14.C29 — Rings + queues + fences + DMA

**Status: QUALIFIED / FROZEN ✅**

Established the GTT-backed SDMA ring, submission queue lifecycle, GTT timeline fences, typed SDMA COPY/FENCE packet model, owned-memory DMA/reference executor, and source-reviewed GFX12 SDMA queue authority plan.

### K14.C30 — Complete basic display engine

**Status: QUALIFIED / FROZEN ✅**

Established EDID parsing/mode selection, connector/CRTC/plane ownership, double-buffered GTT scanout, verified live GOP framebuffer presents/page flips, atomic current-mode commit/rollback, hotplug bookkeeping, and DCN401 resource authority modeling.

Native DCN MMIO programming and physical HPD are not falsely claimed by the QEMU qualification path.

### K14.C31 — Graphics + compute execution

**Status: QUALIFIED / FROZEN ✅**

Established owned shader/resource upload, typed command buffers, separate compute and graphics queues, verified vector-add compute execution, verified triangle draw/live framebuffer presentation, timeline-fence retirement, shader cache/precache, capability reporting, and the corrected `TWSH` shader wire-header path.

Physical Radeon CP/GFX queue execution and native AMD shader ISA remain separately gated where physical evidence is required.

### K14.C32 — Production/stability + final K14

**Status: QUALIFIED / FROZEN ✅**

Final qualification passed:

- queue wrap/stability stress,
- GTT memory pressure/reclaim and conditional VRAM pressure,
- deliberate stuck-queue detection/recovery,
- software interrupt and recovery stress,
- graphics+compute coexistence,
- display+compute coexistence,
- repeated scanout/presentation stability,
- multi-display framework checks,
- PCI multi-GPU inventory groundwork,
- bounded telemetry/performance diagnostics,
- software power policy,
- shader-precache contract freeze,
- syscall-43 userspace GPU ABI/capability freeze,
- stable `displayd` reporting,
- stable userspace handoff,
- intentional halt and final K14 completion markers.

The QEMU status intentionally keeps physical Radeon stress qualification separate rather than inheriting it from the reference backend.

---

## K14 result

K14 is now a **frozen foundation**, not an active milestone.

Future graphics work may build above K14, but qualified K14 behavior must not be silently rewritten. Changes to frozen paths are limited to qualification/status metadata and forward-compatible regression fixes that do not invalidate the qualified contract.

## Next locked milestone

# K15 — ForgeAudio 🔊

K15 begins Titanweave's native low-latency audio foundation.

See `README.md`, `PROJECT_VISION.md`, `BUILD_STATUS.md`, and `COMPLETION_MATRIX.md` for the current project-level view.

---

## Historical K14 note

K14.A through K14.C25 are preserved as qualified/frozen historical bring-up steps that led to the C26-C32 closure sequence. Earlier status text that described one of those historical steps as the active or final milestone is superseded by the owner-locked roadmap and the successful K14.C32 final qualification.