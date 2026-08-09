#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c10-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C10 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C9NF] K14.C9 verified Radeon profiles ready:'
 '[C10WL] per-IP MMIO whitelist engine:'
 '[C10RD] live-read activation policy:'
 '[C10HW] live Radeon MMIO reads:'
 '[C10NF] K14.C10 per-IP MMIO whitelist engine ready:'
 '[C10OK] K14.C10 guarded MMIO-read engine:'
 '[USER] [displayd] K14.C10 per-IP MMIO whitelist and guarded live-read engine online'
 '[USER] [displayd] K14.C10 no physical Radeon in QEMU; MMIO whitelist activation remains deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C10 alive:'
 '[QUAL] K14.C10 guarded Radeon MMIO-read runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi; done
if grep -Fq '[FAIL] K14.C10 per-IP MMIO whitelist/live-read gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C10 guarded-MMIO/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C10 guarded-MMIO/runtime qualification PASSED.'
