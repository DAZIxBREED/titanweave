#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c5-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C5 serial log not found: $LOG" >&2; exit 1; }
markers=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C4NF] K14.C4 Radeon exact-domain qualification ready:'
 '[C5PT] AMD-Vi translation layout:'
 '[C5CB] AMD-Vi command/event policy:'
 '[C5HW] physical AMD-Vi domain image:'
 '[C5RD] K14.C5 AMD-Vi page-table engine ready:'
 '[C5NF] K14.C5 AMD-Vi page-table engine ready:'
 '[USER] [displayd] K14.C5 AMD-Vi page-table engine foundation online'
 '[USER] [displayd] K14.C5 no physical Radeon in QEMU; AMD-Vi tables and requester binding remain deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C5 alive:'
 '[QUAL] K14.C5 AMD-Vi page-table runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for m in "${markers[@]}"; do if grep -Fq "$m" "$LOG"; then echo "PASS  $m"; else echo "MISS  $m"; failed=1; fi; done
if grep -Fq '[FAIL] K14.C5 AMD-Vi page-table engine failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C5 AMD-Vi page-table/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C5 AMD-Vi page-table/runtime qualification PASSED.'
