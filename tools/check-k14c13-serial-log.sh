#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c13-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C13 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C12OK] K14.C12 trusted IP-base/live-read engine:'
 '[C13PV] physical read-proof policy:'
 '[C13HW] physical Radeon qualification:'
 '[C13RD] K14.C13 physical read-proof ready:'
 '[C13OK] K14.C13 physical Radeon read-proof engine:'
 '[USER] [displayd] K14.C13 physical Radeon read-proof qualification gate online'
 '[USER] [displayd] K14.C13 no physical Radeon in QEMU; bare-metal read proof remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C13 alive:'
 '[QUAL] K14.C13 physical-read-proof runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
  if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C13 physical Radeon read-proof gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C13 physical-read-proof runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C13 physical-read-proof runtime qualification PASSED.'
