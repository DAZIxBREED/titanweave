# Titanweave Completion Matrix

- K11: qualified/frozen
- K12: qualified/frozen
- K13.A-D: qualified/frozen
- K14.A: qualified/frozen
- K14.B: qualified/frozen
- K14.C1: qualified/frozen
- K14.C2: qualified/frozen
- K14.C3: source-integrated; QEMU staging qualification pending
- K14.D: pending


## K14.C12
Trusted Radeon IP-base sources and first bounded live status-read path. QEMU qualification pending. Write-side Radeon paths remain fenced.

## K14.C21
Qualified/frozen. Exact generated GFX12 `SCRATCH_REG0` target rebind and bounded identity-write gate. Intentional-HALT QEMU harness termination is frozen into the baseline.

## K14.C22
Qualified/frozen. First bounded reversible non-identity GFX12 `SCRATCH_REG0` mutation: internally derived one-bit probe, exact readback, mandatory original-value restoration, one bounded restore retry, and Radeon bus-master fencing.

## K14.C23
Qualified/frozen. Requires the C22-restored value to persist, then performs two distinct internally-derived one-bit probe/readback/restore cycles on the exact same checksum-backed `SCRATCH_REG0` target. Fedora/QEMU qualification passed; physical Radeon execution remains a separate bare-metal proof.


## K14.C24
Qualified/frozen. Requires frozen C23 stability, then performs one internally-derived four-bit pattern/readback/restore cycle on the exact same checksum-backed `SCRATCH_REG0` target. Fedora/QEMU qualification passed through `[C24OK]`, userspace handoff, `[QUAL]`, and intentional `[HALT]` with automatic QEMU termination. Arbitrary MMIO writes, caller-selected values/addresses, firmware upload, command submission, BAR resizing, MM_INDEX fallback, and bus-master enable remain fenced.

## K14.C25
Qualified/frozen. Requires frozen C24 four-bit pattern/restoration proof, then performs two distinct internally-derived four-bit pattern/readback/restore cycles with an explicit inter-cycle persistence check on the exact same checksum-backed `SCRATCH_REG0` target. User-confirmed Fedora/QEMU qualification freezes the path.

## K14.C26 — Safe Radeon hardware/MMIO foundation
Qualified/frozen. Fedora/QEMU passed exact reviewed GFX12 `SCRATCH_REG1` (`0x2041`, BASE_IDX 1) resolution from the same checksum-qualified GC base slot, the two-entry REG0/REG1 MMIO allowlist, bounded REG1 reads with zero C26 MMIO writes and intentional `[HALT]`. Historical C26 qualification evidence is preserved; the locked roadmap continues Radeon K14 through C32.

## K14.C27 — Complete Radeon driver core
Qualified/frozen. Fedora/QEMU passed the operational driver object/lifecycle, exact ForgeBus ownership, live resource topology, permanent reviewed-MMIO read service, executable error/reset coordination, real masked interrupt route/handler self-test, userspace handoff and intentional halt. Zero placeholders, zero new registers, zero new MMIO writes. Physical Radeon paths remain a separate bare-metal proof. K15 remains ForgeAudio after final K14.C32.


## K14.C28 — Memory + firmware + recovery
Qualified/frozen. Fedora/QEMU passed the operational GTT allocation/map/reclaim, VRAM reservation, GPU-VA reservation, AMD firmware parse/CRC/SHA staging, watchdog/resource-safe software recovery, userspace handoff, `[QUAL]`, and intentional `[HALT]`. Physical silicon upload/reset and C29 execution/DMA authority remain fenced.


## K14.C29 — Rings + queues + fences + DMA
Qualified/frozen. Fedora/QEMU runtime qualification passed. Real GTT ring backing, FIFO submission lifecycle, GTT timeline fences, typed source-backed SDMA COPY/FENCE packets, and bounded owned-memory DMA execution are implemented. Physical Radeon SDMA remains fail-closed behind real firmware/translation/IOMMU prerequisites.
