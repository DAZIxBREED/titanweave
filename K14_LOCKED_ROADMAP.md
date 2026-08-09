# Titanweave K14 Locked Radeon Roadmap

Status: **SET IN STONE BY PROJECT OWNER**

This roadmap is authoritative for the remainder of K14. It must not be renamed, reordered, expanded into additional C milestones, or moved into K15 unless the project owner explicitly changes it.

- **K14.C26 — Safe hardware/MMIO foundation** — qualified/frozen.
- **K14.C27 — Complete Radeon driver core** — **qualified/frozen**. Device/lifecycle, capability model, driver-owned reviewed-MMIO service, ForgeBus/resource ownership, error/reset coordination, interrupt route/handler, VRAM/GTT topology and diagnostics.
- **K14.C28 — Memory + firmware + recovery** — **qualified/frozen**. VRAM manager, GTT/system memory, GPU VA groundwork, buffer backing/mapping, firmware validation/loading, reset/watchdog/recovery and required interrupt activation.
- **K14.C29 — Rings + queues + fences + DMA** — **qualified/frozen**. Command rings, queue ownership, fences/synchronization, SDMA/DMA, bounded real command completion and submission validation.
- **K14.C30 — Complete basic display engine** — **qualified/frozen**. Connector/EDID/mode discovery, display pipes, framebuffer scanout, modesetting, page flip, hotplug and multi-display foundation.
- **K14.C31 — Graphics + compute execution** — **qualified/frozen**. Graphics/compute queues, shader/resource upload, pipeline/command encoding, verified compute dispatch and graphics draw, shader-cache/precache hooks.
- **K14.C32 — Production/stability + final K14** — **source-integrated; runtime qualification pending**. Queue/memory/interrupt/recovery stress, display/compute and graphics/compute coexistence, repeated display stability, diagnostics, power policy, userspace GPU ABI/capability freeze, shader-precache freeze, multi-GPU enumeration groundwork, and final Radeon foundation qualification.

After K14.C32 is qualified/frozen:

- **K14 — COMPLETE / FROZEN**
- **K15 — ForgeAudio**

## No-stub rule

C27-C32 may contain internal gates, but those gates do not become additional C milestones. No subsystem may be represented by a TODO, `unimplemented!()`, placeholder implementation, fake success path, or nonfunctional API shell. A feature that cannot be implemented to its declared milestone scope is omitted until the milestone that owns it.
