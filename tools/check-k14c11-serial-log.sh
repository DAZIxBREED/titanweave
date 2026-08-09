#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c11-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C11 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C10NF] K14.C10 per-IP MMIO whitelist engine ready:'
 '[C11RF] reviewed Radeon register definitions:'
 '[C11BA] IP-base address resolver:'
 '[C11HW] physical Radeon register reads:'
 '[C11RD] K14.C11 reviewed register whitelist ready:'
 '[C11OK] K14.C11 reviewed register/IP-base gate:'
 '[USER] [displayd] K14.C11 reviewed Radeon register definitions and IP-base resolver gate online'
 '[USER] [displayd] K14.C11 no physical Radeon in QEMU; reviewed register definitions remain safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C11 alive:'
 '[QUAL] K14.C11 reviewed-register runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
  if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C11 reviewed register/IP-base gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C11 reviewed-register/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C11 reviewed-register/runtime qualification PASSED.'
