#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c9-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C9 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C8NF] K14.C8 Radeon ASIC/IP identification ready:'
 '[C9PF] verified Radeon profile table:'
 '[C9PR] live safe-read policy:'
 '[C9IP] profile promotion:'
 '[C9HW] live Radeon profile verification:'
 '[C9RD] K14.C9 verified Radeon profiles ready:'
 '[C9NF] K14.C9 verified Radeon profiles ready:'
 '[USER] [displayd] K14.C9 verified Radeon profiles and live safe-identity-read gate online'
 '[USER] [displayd] K14.C9 no physical Radeon in QEMU; profile/live PCI reads remain deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C9 alive:'
 '[QUAL] K14.C9 verified Radeon profile runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
  if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C9 verified Radeon profile/live-read gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C9 verified-profile/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C9 verified-profile/runtime qualification PASSED.'
