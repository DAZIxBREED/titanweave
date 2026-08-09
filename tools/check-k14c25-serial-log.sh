#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c25-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C25 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C24OK] K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate:'
 '[C25DP] dual multi-bit patterns:'
 '[C25PG] dual-pattern policy:'
 '[C25TX] dual-pattern stability contract:'
 '[C25HW] GFX12 SCRATCH_REG0 dual multi-bit pattern stability:'
 '[C25RD] K14.C25 dual-pattern ready:'
 '[C25OK] K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate:'
 '[USER] [displayd] K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate online'
 '[USER] [displayd] K14.C25 no physical Radeon in QEMU; dual multi-bit pattern stability transaction remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C25 alive:'
 '[QUAL] K14.C25 dual-multi-bit-pattern runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C25 GFX12 dual multi-bit pattern stability gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C25 dual-multi-bit-pattern runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C25 dual-multi-bit-pattern runtime qualification PASSED.'
