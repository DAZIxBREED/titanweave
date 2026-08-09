#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c4-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C4 serial log not found: $LOG" >&2; exit 1; }
markers=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C3NF] K14.C3 Radeon bare-metal staging ready:'
 '[C4IV] AMD-Vi exact-requester gate:'
 '[C4DM] Radeon domain policy:'
 '[C4AP] Radeon aperture promotion policy:'
 '[C4HW] physical Radeon domain bind:'
 '[C4RD] K14.C4 Radeon exact-domain gate ready:'
 '[C4NF] K14.C4 Radeon exact-domain qualification ready:'
 '[USER] [displayd] K14.C4 exact Radeon requester/AMD-Vi gate online'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C4 alive:'
 '[QUAL] K14.C4 Radeon domain-gate runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for m in "${markers[@]}"; do
  if grep -Fq "$m" "$LOG"; then echo "PASS  $m"; else echo "MISS  $m"; failed=1; fi
done
if grep -Fq '[FAIL] K14.C4 Radeon exact-domain qualification failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C4 Radeon domain-gate/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C4 Radeon domain-gate/runtime qualification PASSED.'
