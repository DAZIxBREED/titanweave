#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c22-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C22 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C21OK] K14.C21 reviewed GFX12 target rebind/identity-write gate:'
 '[C22RV] reversible GFX12 scratch mutation:'
 '[C22PG] reversible-write policy:'
 '[C22TX] reversible transaction contract:'
 '[C22HW] GFX12 SCRATCH_REG0 reversible mutation:'
 '[C22RD] K14.C22 reversible-write ready:'
 '[C22OK] K14.C22 reversible GFX12 SCRATCH_REG0 mutation gate:'
 '[USER] [displayd] K14.C22 bounded reversible GFX12 SCRATCH_REG0 mutation gate online'
 '[USER] [displayd] K14.C22 no physical Radeon in QEMU; reversible scratch mutation remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C22 alive:'
 '[QUAL] K14.C22 reversible-scratch-mutation runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C22 reversible GFX12 scratch mutation gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C22 reversible-scratch-mutation runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C22 reversible-scratch-mutation runtime qualification PASSED.'
