#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c6-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C6 serial log not found: $LOG" >&2; exit 1; }
markers=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C5NF] K14.C5 AMD-Vi page-table engine ready:'
 '[C6RG] AMD-Vi live register engine:'
 '[C6SQ] AMD-Vi activation sequence:'
 '[C6HW] live AMD-Vi programming:'
 '[C6RD] K14.C6 live AMD-Vi engine ready:'
 '[C6NF] K14.C6 live AMD-Vi engine ready:'
 '[USER] [displayd] K14.C6 live AMD-Vi hardware-programming boundary online'
 '[USER] [displayd] K14.C6 no physical Radeon in QEMU; AMD-Vi hardware programming remains deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C6 alive:'
 '[QUAL] K14.C6 live AMD-Vi runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for m in "${markers[@]}"; do if grep -Fq "$m" "$LOG"; then echo "PASS  $m"; else echo "FAIL  $m" >&2; failed=1; fi; done
if grep -Fq '[FAIL] K14.C6 live AMD-Vi engine failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C6 live AMD-Vi/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C6 live AMD-Vi/runtime qualification PASSED.'
