#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c31-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C31 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C30OK] K14.C30 complete basic display engine:'
 '[C31SH] shader/resource model:'
 '[C31CQ] compute queue:'
 '[C31CP] compute execution:'
 '[C31GQ] graphics queue:'
 '[C31GX] graphics execution:'
 '[C31HC] compute capability model:'
 '[C31SC] shader cache/precache:'
 '[C31PG] execution authority:'
 '[C31RD] K14.C31 graphics+compute execution ready:'
 '[C31OK] K14.C31 graphics+compute execution:'
 '[USER] [displayd] K14.C31 graphics and compute execution subsystem online'
 '[USER] [displayd] K14.C31 QEMU reference backend verified: owned shader upload/cache, typed commands, separate compute/graphics queues, vector-add dispatch, triangle draw, timeline fences, and live framebuffer present operational'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio'
 '[KERN] K14.C31 alive:'
 '[QUAL] K14.C31 graphics-compute-execution runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi; done
if grep -Fq '[FAIL] K14.C31 graphics+compute execution failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C31 graphics-compute-execution runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C31 graphics-compute-execution runtime qualification PASSED.'
