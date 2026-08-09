#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c8-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C8 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C7NF] K14.C7 Radeon discovery ready:'
 '[C8ID] Radeon ASIC/IP identity gate:'
 '[C8RR] safe register-read policy:'
 '[C8IP] IP manifest policy:'
 '[C8HW] physical Radeon identification:'
 '[C8RD] K14.C8 Radeon ASIC/IP identification ready:'
 '[C8NF] K14.C8 Radeon ASIC/IP identification ready:'
 '[USER] [displayd] K14.C8 Radeon ASIC/IP identification and safe-register-read gate online'
 '[USER] [displayd] K14.C8 no physical Radeon in QEMU; ASIC profile and register reads remain deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C8 alive:'
 '[QUAL] K14.C8 Radeon ASIC/IP runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
  if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C8 Radeon ASIC/IP identification failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C8 Radeon ASIC-IP/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C8 Radeon ASIC-IP/runtime qualification PASSED.'
