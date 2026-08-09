#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c7-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C7 serial log not found: $LOG" >&2; exit 1; }
markers=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C6NF] K14.C6 live AMD-Vi engine ready:'
 '[C7AP] Radeon read-only aperture policy:'
 '[C7FW] Radeon discovery sequence:'
 '[C7GT] GMC/GTT promotion policy:'
 '[C7HW] physical Radeon discovery:'
 '[C7RD] K14.C7 Radeon discovery ready:'
 '[C7NF] K14.C7 Radeon discovery ready:'
 '[USER] [displayd] K14.C7 Radeon MMIO/firmware discovery staging online'
 '[USER] [displayd] K14.C7 no physical Radeon in QEMU; read-only MMIO and firmware discovery remain deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C7 alive:'
 '[QUAL] K14.C7 Radeon discovery runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for m in "${markers[@]}"; do if grep -Fq "$m" "$LOG"; then echo "PASS  $m"; else echo "FAIL  $m" >&2; failed=1; fi; done
if grep -Fq '[FAIL] K14.C7 Radeon discovery failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C7 Radeon discovery/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C7 Radeon discovery/runtime qualification PASSED.'
