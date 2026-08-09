#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c12-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C12 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C11OK] K14.C11 reviewed register/IP-base gate:'
 '[C12BS] trusted Radeon IP-base source:'
 '[C12RM] Radeon register aperture:'
 '[C12LR] live-read sequence:'
 '[C12HW] physical Radeon status reads:'
 '[C12RD] K14.C12 trusted IP-base/live-read path ready:'
 '[C12OK] K14.C12 trusted IP-base/live-read engine:'
 '[USER] [displayd] K14.C12 trusted Radeon IP-base and live status-read gate online'
 '[USER] [displayd] K14.C12 no physical Radeon in QEMU; trusted-base/live-read path remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C12 alive:'
 '[QUAL] K14.C12 trusted-base/live-read runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
  if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C12 trusted IP-base/live-read gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C12 trusted-base/live-read runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C12 trusted-base/live-read runtime qualification PASSED.'
