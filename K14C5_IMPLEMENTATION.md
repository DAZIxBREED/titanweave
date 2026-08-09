# Titanweave K14.C5 — AMD-Vi Page-Table Engine Foundation

K14.C5 forks from the qualified K14.C4 baseline. It turns the exact-Radeon requester gate into concrete pinned AMD-Vi translation structures while preserving fail-closed behavior.

## Implemented in this slice

- 32-byte per-requester AMD-Vi device-table image covering all 65,536 requester IDs.
- Pinned second-level page-table root for the selected Radeon domain.
- Pinned bounded command buffer and event log images.
- Exact BDF/RID DTE image tied to domain `0x14c5`.
- Event/fault path readiness state and DISPLAYD query ABI.
- QEMU regression path that proves no surrogate can promote physical-Radeon DMA.

## Safety boundary

C5 deliberately does **not** claim the physical AMD-Vi unit is programmed merely because these structures exist. Register programming, command completion, event-log consumption and a live persistent domain require bare-metal qualification against the discovered IVRS unit. Until then, Radeon bus mastering, MMIO writes, firmware upload and command submission remain off.

The next C5 bare-metal substep is to version/capability-check the AMD-Vi MMIO block, program device-table/command/event base registers, enable the IOMMU, issue an exact-domain invalidation/completion command, and prove a controlled translated DMA transaction or fault before any Radeon bus mastering is admitted.
