# Titanweave K14.C6 — Live AMD-Vi Hardware Programming Boundary

K14.C6 forks frozen K14.C5 and adds the physical AMD-Vi register-programming engine boundary. It introduces the AMD-Vi MMIO register model, a validated `AmdViHardwarePlan`, exact-requester activation sequencing, command/event ring publication, translation-enable proof state, invalidation/fault gates, and promotion rules for a persistent Radeon DMA domain.

## Fail-closed rule

The code contains the physical programming primitive, but `AMDVI_C6_ENABLE_BARE_METAL_PROGRAMMING` remains false in the QEMU-qualified source. QEMU uses Intel VT-d and has no physical Radeon, so it must never activate AMD-Vi hardware. A later bare-metal qualification build may arm this gate only after the host inventory proves the IVHD unit, exact Radeon requester, and C5 pinned structures. Radeon bus mastering, Radeon MMIO writes, firmware upload, and command submission remain disabled.

## C6 sequence

1. Prove physical AMD Radeon and exact requester.
2. Prove active AMD-Vi/IVRS unit and register base.
3. Reuse the pinned C5 device table, page root, command buffer, and event log.
4. Validate the hardware programming plan.
5. Map AMD-Vi MMIO through the kernel-only MMIO window.
6. Initialize command/event indices, publish table/ring bases, enable command/event processing, then translation.
7. Require invalidation/completion and fault-path proof before marking the domain persistent.
8. Promote read-only Radeon MMIO only after the translated domain is live.

K14.C6 does not enable the Radeon PCI bus-master bit.
