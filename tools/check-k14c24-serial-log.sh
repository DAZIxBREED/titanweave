#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c24-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C24 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C23OK] K14.C23 GFX12 SCRATCH_REG0 persistence/dual-probe stability gate:'
 '[C24PT] reversible multi-bit pattern:'
 '[C24PG] multi-bit-write policy:'
 '[C24TX] reversible pattern contract:'
 '[C24HW] GFX12 SCRATCH_REG0 reversible multi-bit pattern:'
 '[C24RD] K14.C24 multi-bit-pattern ready:'
 '[C24OK] K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate:'
 '[USER] [displayd] K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate online'
 '[USER] [displayd] K14.C24 no physical Radeon in QEMU; reversible multi-bit pattern transaction remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C24 alive:'
 '[QUAL] K14.C24 reversible-multi-bit-pattern runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C24 reversible GFX12 multi-bit pattern gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C24 reversible-multi-bit-pattern runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C24 reversible-multi-bit-pattern runtime qualification PASSED.'
