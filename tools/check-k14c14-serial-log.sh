#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c14-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C14 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C13OK] K14.C13 physical Radeon read-proof engine:'
 '[C14PG] write-promotion policy:'
 '[C14HW] Radeon write-promotion readiness:'
 '[C14RD] K14.C14 write-promotion readiness ready:'
 '[C14OK] K14.C14 controlled write-promotion readiness gate:'
 '[USER] [displayd] K14.C14 controlled Radeon write-promotion readiness gate online'
 '[USER] [displayd] K14.C14 no physical Radeon in QEMU; write-promotion readiness remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C14 alive:'
 '[QUAL] K14.C14 write-promotion-readiness runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
  if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C14 write-promotion readiness gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C14 write-promotion-readiness runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C14 write-promotion-readiness runtime qualification PASSED.'
