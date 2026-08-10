# Titanweave Build Status

K14.C5 is source-integrated from the frozen, QEMU-qualified K14.C4 baseline. All inherited K1-K14.C4 source gates and the new K14.C5 AMD-Vi page-table gate pass in the packaging environment. Userspace assembly builds successfully here. The packaging environment does not contain Cargo/Rust, so full Rust/QEMU runtime qualification must be performed on Fedora before C5 is frozen. Physical Radeon bus mastering, MMIO writes, firmware upload and command submission remain fenced.

K14.C22: qualified/frozen. Bounded reversible GFX12 SCRATCH_REG0 mutation and exact restoration passed Fedora/QEMU qualification; physical Radeon execution remains a separate bare-metal proof.

K14.C23: qualified/frozen. Fedora/QEMU passed the post-restore persistence and dual-probe stability safe-defer path with automatic intentional-HALT termination.

K14.C24: qualified/frozen from frozen qualified K14.C23. Fedora/QEMU runtime qualification passed the deterministic reversible four-bit SCRATCH_REG0 pattern/readback/restore gate with automatic intentional-HALT termination. Physical Radeon execution remains a separate bare-metal proof.

K14.C25: qualified/frozen from frozen qualified K14.C24. The dual deterministic four-bit SCRATCH_REG0 stability path is frozen by user-confirmed Fedora/QEMU qualification. No additional Radeon register authority was opened.

K14.C26: qualified/frozen Radeon MMIO foundation. Fedora/QEMU passed exact GFX12 SCRATCH_REG1 (`0x2041`, BASE_IDX 1) resolution, the two-entry reviewed REG0/REG1 MMIO allowlist, bounded read-only REG1 proof with zero C26 MMIO writes, userspace handoff, the historical C26 closure marker, and intentional `[HALT]`. The project owner subsequently locked K14 to continue through C32; K15 remains ForgeAudio.

K14.C27: qualified/frozen operational Radeon driver core. Fedora/QEMU passed the complete C27 driver-core software path, including lifecycle, ForgeBus ownership, live resource/topology, reviewed-MMIO service, executable error/reset coordination, real interrupt-route/handler self-test, userspace handoff, intentional halt, and the C26 foundation-continuation marker. QEMU contains no physical Radeon, so physical ownership/MMIO remains safely deferred. No placeholders/stubs are permitted. C27 adds zero new registers/writes and leaves firmware, DMA/bus mastering, command submission and physical interrupt enable fenced.


K14.C28: qualified/frozen from frozen C27. Fedora/QEMU passed the operational memory, firmware-validation/staging, watchdog/recovery, userspace handoff, and intentional-halt path. QEMU has no physical Radeon, so silicon firmware upload, physical ASIC reset, GPU page tables, Radeon DMA/bus mastering, rings/queues, command submission, and physical GPU interrupt programming remain unclaimed and fenced for later milestones. No stubs/placeholders are allowed.


K14.C29: qualified/frozen from frozen C28. Fedora/QEMU passed rings + queues + fences + DMA software/runtime qualification; physical Radeon SDMA execution remains a separate bare-metal proof. Implements the operational GTT-backed SDMA ring, FIFO submission queue, timeline fence, typed SDMA COPY/FENCE codec, owned-memory copy/fence executor, and exact GFX12 SDMA0 queue-0 register plan. Physical SDMA/bus-master activation remains fail-closed until firmware-in-silicon, GPU translation and a persistent translated Radeon IOMMU domain are live. No stubs or raw packet/MMIO authority are allowed.

K14.C30: qualified/frozen from frozen C29. Fedora/QEMU passed EDID/mode selection, connector/CRTC/plane ownership, double-buffered GTT scanout, live GOP framebuffer page flips, atomic rollback, hotplug bookkeeping, userspace handoff and intentional HALT. Native DCN programming/physical HPD remain separately gated.

K14.C31: qualified/frozen from frozen C30. Fedora/QEMU passed owned shader upload/cache/precache, typed command buffers, separate compute/graphics queues, verified vector-add dispatch, verified triangle draw/live framebuffer present, timeline fences, userspace handoff, intentional HALT and the corrected shader wire-magic path. Physical Radeon CP/GFX queues and native AMD ISA remain separately gated.

K14.C32: qualified/frozen final K14 Radeon production/stability baseline. Fedora/QEMU qualification completed the final production/stability stress, hang recovery, memory pressure/reclaim, interrupt/recovery stress, display+compute and graphics+compute coexistence, repeated display presents, multi-display and multi-GPU inventory groundwork, bounded telemetry, software power policy, shader-precache freeze, syscall-43 userspace GPU ABI/capability freeze, and strict physical-evidence separation. K14 is complete; physical Radeon stress remains separately evidenced on bare metal.

K15.1: source-integrated from the frozen/qualified K14.C32 baseline under the locked 16-gate ForgeAudio stone contract. Implements a real `RealtimeAudio` scheduler class, 1 kHz qualification tick, fixed-priority/deadline dispatch, bounded period budgets, deadline tracking, CPU affinity plus an explicit ForgeAudio CPU reservation, a bounded priority-inheriting sleepable RT mutex, bounded preemption guards, temporary-task stack reclamation, an eight-job 4 ms periodic audio workload and competing normal load. All inherited K1-K14 source regressions plus K15.1 source checks pass in the packaging environment. Rust/Cargo and QEMU are unavailable here, so compile/runtime qualification remains pending on the Fedora development host. K15.2 is blocked until K15.1 runtime qualification passes.

K15.1: qualified/frozen. Fedora/QEMU completed the ForgeAudio RT qualification with exactly eight periodic jobs, zero deadline misses/budget exhaustions/guard overruns, one priority-inheritance event and one bounded-preemption deferral while preserving K14.C32 through intentional HALT.

K15.2: source-integrated from frozen K15.1. Adds shared ForgeAudio ABI v1, real bounded device/endpoint/stream/buffer/clock/event/fence lifecycle, rights-bearing audio handles, syscall 44/45/46, real bounded buffer memory/readback, monotonic clocks/fences, bounded event FIFO and strict stream-state/recovery logic. QEMU hardware enumeration remains honestly empty; no placeholder audio device is created. Runtime qualification remains pending on Fedora before K15.3 may begin.

K15.2: qualified/frozen. Fedora/QEMU passed ForgeAudio ABI v1 with honest zero-device hardware enumeration, strict stream lifecycle/recovery, real bounded buffer memory/readback, monotonic clock/event/fence objects, inherited K15.1 and K14.C32 qualification, and intentional HALT.

K15.3: source-integrated from frozen K15.2. Adds real contiguous DMA-ring allocation, kernel DMA mapping/teardown, bounded cyclic periods, playback/capture ownership, position/wrap accounting, translated-IOMMU hardware-arm gating, IOVA device addresses and XRUN detection. The self-test exercises real mapped memory and the transport state machine while explicitly deferring HDA hardware/IRQ evidence to K15.4. Runtime qualification remains pending on Fedora before K15.4 may begin.
