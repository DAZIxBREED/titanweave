#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c20-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C20 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C19OK] K14.C19 physical AMD discovery snapshot gate:'
 '[C20IP] AMD live IP-record resolver:'
 '[C20PG] exact-base policy:'
 '[C20HW] AMD exact IP bases:'
 '[C20RD] K14.C20 exact-base gate ready:'
 '[C20OK] K14.C20 AMD exact live IP-base gate:'
 '[USER] [displayd] K14.C20 AMD exact live IP-base resolver online'
 '[USER] [displayd] K14.C20 no physical Radeon in QEMU; verified-snapshot IP-base resolution remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C20 alive:'
 '[QUAL] K14.C20 exact-IP-base runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C20 AMD exact live IP-base gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C20 exact-IP-base runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C20 exact-IP-base runtime qualification PASSED.'
