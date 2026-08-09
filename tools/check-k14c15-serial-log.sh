#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c15-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C15 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C14OK] K14.C14 controlled write-promotion readiness gate:'
 '[C15TX] controlled-write policy:'
 '[C15HW] controlled Radeon write transaction:'
 '[C15RD] K14.C15 controlled write transaction ready:'
 '[C15OK] K14.C15 controlled write transaction:'
 '[USER] [displayd] K14.C15 controlled Radeon write transaction gate online'
 '[USER] [displayd] K14.C15 no physical Radeon in QEMU; controlled identity-write transaction remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C15 alive:'
 '[QUAL] K14.C15 controlled-write runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
  if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C15 controlled write transaction failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C15 controlled-write runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C15 controlled-write runtime qualification PASSED.'
