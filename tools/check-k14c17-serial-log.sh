#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c17-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C17 serial log not found: $LOG" >&2; exit 1; }
markers=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C16OK] K14.C16 reviewed Radeon MMIO-write gate:'
 '[C17IP] AMD IP-discovery parser:'
 '[C17PG] Navi48 resolution policy:'
 '[C17HW] Navi48 IP discovery:'
 '[C17RD] K14.C17 IP-discovery gate ready:'
 '[C17OK] K14.C17 AMD IP-discovery/Navi48 base-resolution gate:'
 '[USER] [displayd] K14.C17 AMD IP-discovery/Navi48 base-resolution gate online'
 '[USER] [displayd] K14.C17 no physical Radeon in QEMU; live IP-discovery fetch remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C17 alive:'
 '[QUAL] K14.C17 IP-discovery runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for m in "${markers[@]}"; do if grep -Fq "$m" "$LOG"; then echo "PASS  $m"; else echo "FAIL  $m" >&2; failed=1; fi; done
if grep -Fq '[FAIL] K14.C17 AMD IP-discovery gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C17 IP-discovery runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C17 IP-discovery runtime qualification PASSED.'
