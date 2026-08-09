# K11 Runtime Closure

This pass activates the K11.0 prerequisites that previously existed only as disconnected modules.

Implemented wiring:

- ForgeBus registries persist for the lifetime of the kernel.
- Process exit/fault performs deferred teardown after switching away from the user CR3.
- User pages and page tables are reclaimed; kernel stacks are reclaimed from a safe stack.
- Process handles and owned lifecycle records are released.
- Block I/O cancellation is two-phase: active DMA becomes cancel/timeout-pending and buffers remain pinned until a device fence completes.
- Watchdog actions now execute unbind, interrupt masking, DMA teardown, request fencing, restart, or quarantine.
- Device vectors 0x50-0xDF have actual IDT stubs and dispatch accounting.
- Physical allocator counters only include extents successfully represented in its metadata table.

Hardware-specific reset and bus-master-disable sequences remain the responsibility of each K11.1+ backend before it calls ForgeBus recovery/fence completion.
