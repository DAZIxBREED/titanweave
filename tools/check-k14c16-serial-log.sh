#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c16-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C16 serial log not found: $LOG" >&2; exit 1; }
markers=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C15OK] K14.C15 controlled write transaction:'
 '[C16RV] reviewed MMIO target:'
 '[C16PG] MMIO-write policy:'
 '[C16HW] Radeon MMIO identity-write:'
 '[C16RD] K14.C16 reviewed MMIO-write gate ready:'
 '[C16OK] K14.C16 reviewed Radeon MMIO-write gate:'
 '[USER] [displayd] K14.C16 reviewed Radeon MMIO-write transaction gate online'
 '[USER] [displayd] K14.C16 no physical Radeon in QEMU; reviewed MMIO identity-write remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C16 alive:'
 '[QUAL] K14.C16 reviewed-MMIO-write runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for m in "${markers[@]}"; do if grep -Fq "$m" "$LOG"; then echo "PASS  $m"; else echo "FAIL  $m"; failed=1; fi; done
if grep -Fq '[FAIL] K14.C16 reviewed Radeon MMIO-write gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C16 reviewed-MMIO-write runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C16 reviewed-MMIO-write runtime qualification PASSED.'
