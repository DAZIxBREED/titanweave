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

K14.C30: source-integrated from frozen C29; runtime qualification pending. Implements validated EDID/mode selection, bounded connector/CRTC/plane ownership, double-buffered pinned GTT scanout, real volatile GOP framebuffer presents/page flips, atomic current-mode rollback, hotplug bookkeeping, and source-reviewed DCN401 resource capabilities. Native DCN programming/physical HPD remain fail-closed. No stubs/placeholders are allowed.
