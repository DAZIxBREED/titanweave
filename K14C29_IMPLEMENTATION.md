# Titanweave K14.C29 — Rings + Queues + Fences + DMA

Status: **IMPLEMENTED / SOURCE-VALIDATED / RUNTIME QUALIFICATION PENDING**

K14.C29 is the locked-roadmap execution-control milestone after frozen K14.C28. It implements a real GTT-backed SDMA ring, an explicit FIFO submission queue with lifecycle/cancellation, a GTT-backed timeline fence, typed SDMA COPY_LINEAR/FENCE packet codecs, and an executable bounded DMA qualification over real C28-owned mapped memory.

## Implemented execution path

- `radeon_ring.rs`: 16 KiB reclaimable/pinnable C28 GTT ring, volatile dword emission, wrap accounting, full detection, 16-dword commit alignment and NOP padding.
- `radeon_queue.rs`: 32-entry FIFO submission queue with Queued -> Emitted -> Retired transitions, fence-ordered retirement, cancellation, counter validation and overflow rejection.
- `radeon_fence.rs`: real GTT timeline memory, monotonic sequence issue, volatile completion readback and exact UC SDMA FENCE packet generation.
- `radeon_dma.rs`: allocates real source/destination GTT objects, writes a deterministic pattern, emits the same typed COPY_LINEAR + FENCE stream used by the ring, decodes that stream through the bounded C29 executor, performs the copy over the owned mappings, retires the fence, and verifies every byte.
- `radeon_sdma.rs`: exact GFX12 SDMA0 queue-0 register identities rooted at GC BASE_IDX 0. When a checksum-qualified AMD discovery snapshot exists, the exact GC base-0 is re-resolved through the frozen C21 resolver.
- `native_gpu_c29.rs`: integrates all C29 gates, preserves the physical-Radeon fail-closed boundary and publishes syscall/userspace diagnostics.

## Source-backed SDMA definitions

The C29 definitions follow the upstream AMD/Linux GFX12 SDMA 7 implementation: SDMA 7 consumes the generated SDMA v6 packet format; COPY=1, FENCE=5, COPY_LINEAR sub-op=0; the linear-copy count is `byte_count - 1`; the SDMA 7 fence uses UC mtype 3. GFX12 queue-0 registers begin at `RB_CNTL=0x0080`, `RB_BASE=0x0081`, `RB_WPTR=0x0085` in GC base slot 0. Upstream SDMA 7 uses 64-bit ring pointers and a 0xf alignment mask.

## Physical hardware boundary

C29 does not lie about prerequisites that C28 intentionally did not provide. Physical SDMA programming and Radeon bus mastering remain fail-closed unless all of these are actually proven live: firmware uploaded into silicon, GPU address translation/page tables, and a persistent hardware-translated IOMMU domain for the Radeon requester. Raw userspace packets, caller-selected MMIO addresses/values, and unprotected bus mastering are forbidden.

This is not a stub: the ring/queue/fence/DMA control plane and packet executor are operational and exercised in QEMU over real Titanweave-owned mapped pages. Physical SDMA execution is separately gated because QEMU has no Radeon and Titanweave does not yet possess the required physical execution prerequisites.
