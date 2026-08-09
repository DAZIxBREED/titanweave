#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c29-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C29 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C28OK] K14.C28 Radeon memory+firmware+recovery:'
 '[C29RG] SDMA ring:'
 '[C29QU] submission queue:'
 '[C29FN] timeline fence:'
 '[C29DM] typed SDMA DMA:'
 '[C29SD] GFX12 SDMA0 queue0 authority:'
 '[C29PG] execution authority:'
 '[C29RD] K14.C29 rings+queues+fences+DMA ready:'
 '[C29OK] K14.C29 Radeon rings+queues+fences+DMA:'
 '[USER] [displayd] K14.C29 Radeon rings, queues, timeline fences, and typed DMA subsystem online'
 '[USER] [displayd] K14.C29 no physical Radeon in QEMU; GTT ring, FIFO queue, SDMA packet codec, timeline fence, and owned-memory DMA executor qualified while physical SDMA remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio'
 '[KERN] K14.C29 alive:'
 '[QUAL] K14.C29 rings-queues-fences-dma runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C29 Radeon rings+queues+fences+DMA failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C29 rings-queues-fences-dma runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C29 rings-queues-fences-dma runtime qualification PASSED.'
