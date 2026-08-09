# Titanweave K14.C27 — Complete Radeon Driver Core

K14.C27 is the first large integrated milestone under the locked C27-C32 roadmap. It converts the frozen C26 MMIO proof chain into persistent, reusable native Radeon driver infrastructure.

## Implemented operational subsystems

### Radeon device/lifecycle core

`kernel/weavecore/src/radeon_driver.rs` implements a concrete `CoreMachine` with validated transitions through claimed, MMIO-ready, online, quiesced and faulted states. Fault accounting, reset epochs and reset coordination are executable state transitions, not placeholders. The self-test exercises normal transitions, fault/reset recovery and rejection of an invalid transition.

### ForgeBus ownership and resource topology

`kernel/weavecore/src/radeon_resources.rs` resolves the exact selected Radeon back to its retained ForgeBus `DeviceId` and bound driver. On physical Radeon it captures the exact PCI BDF, BAR0 VRAM aperture base/visible size, BAR2 base when present, BAR5 MMIO base, legacy IRQ line, memory-decode state, bus-master state, hardware-IOMMU readiness and persistent-domain state. The topology is accepted only when ForgeBus ownership and the frozen C26 safety lineage agree.

`forgebus.rs` now exposes exact PCI-to-retained-device, bound-driver and retained-device-state queries so the Radeon driver uses ForgeBus as the single ownership authority.

### Permanent reviewed-MMIO service

`kernel/weavecore/src/radeon_mmio.rs` replaces milestone-local address use with identity-based access to the two frozen C26 targets. Callers select `ScratchReg0` or `ScratchReg1`; they cannot supply addresses or data. Physical reads use read-only MMIO mappings and recheck PCI memory decode and bus-master fencing. Generic writes are executable policy rejections. C27 adds zero new registers and zero new MMIO writes.

### Interrupt core

C27 registers an actual interrupt route and handler through the existing K11 interrupt router when physical Radeon ownership exists. The route intentionally remains masked because physical MSI/MSI-X/interrupt-source programming belongs to C29. The handler itself is not a stub: C27's self-test allocates a route, proves masked dispatch rejection, enables the software route, dispatches it, executes the handler, records the event, remasks and releases it.

### Error/reset coordination

The C27 driver core contains working fault accounting and reset coordination. This is deliberately not claimed as a physical GPU reset; C28 owns physical reset/recovery. C27 provides the real driver state transitions and accounting that C28 will call when hardware reset is implemented.

## Authority retained from C26

C27 does not widen dangerous hardware authority:

- new reviewed MMIO registers: 0
- new MMIO writes: 0
- generic MMIO writes: forbidden
- caller-supplied MMIO addresses: forbidden
- caller-supplied MMIO values: forbidden
- firmware upload: forbidden
- DMA/bus-master enable: forbidden
- GPU command submission: forbidden
- physical GPU interrupt enable: forbidden

## QEMU qualification

QEMU has no physical Radeon. Therefore QEMU executes and qualifies the complete software driver-core machinery, policy enforcement, lifecycle/error/reset state machine, real interrupt-router handler dispatch self-test, ABI/userspace path and fallback behavior while physical ForgeBus Radeon ownership and MMIO reads remain safely deferred.
