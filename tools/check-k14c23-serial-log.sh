#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c23-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C23 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C22OK] K14.C22 reversible GFX12 SCRATCH_REG0 mutation gate:'
 '[C23PS] post-restore persistence gate:'
 '[C23PG] dual-probe policy:'
 '[C23TX] stability transaction contract:'
 '[C23HW] GFX12 SCRATCH_REG0 dual-probe stability:'
 '[C23RD] K14.C23 stability ready:'
 '[C23OK] K14.C23 GFX12 SCRATCH_REG0 persistence/dual-probe stability gate:'
 '[USER] [displayd] K14.C23 GFX12 SCRATCH_REG0 persistence and dual-probe stability gate online'
 '[USER] [displayd] K14.C23 no physical Radeon in QEMU; dual-probe stability transaction remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C23 alive:'
 '[QUAL] K14.C23 dual-probe-stability runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C23 GFX12 scratch persistence/dual-probe stability gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C23 dual-probe-stability runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C23 dual-probe-stability runtime qualification PASSED.'
