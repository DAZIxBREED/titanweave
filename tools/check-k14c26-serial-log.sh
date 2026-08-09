#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c26-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C26 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C25OK] K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate:'
 '[C26RV] second reviewed GFX12 target:'
 '[C26AL] final K14 MMIO allowlist:'
 '[C26PG] K14 completion policy:'
 '[C26HW] GFX12 SCRATCH_REG1 read proof:'
 '[C26RD] K14.C26 completion ready:'
 '[C26OK] K14.C26 final reviewed GFX12 MMIO allowlist/read-only completion gate:'
 '[USER] [displayd] K14.C26 final GFX12 reviewed MMIO allowlist and SCRATCH_REG1 read-only completion gate online'
 '[USER] [displayd] K14.C26 no physical Radeon in QEMU; final K14 SCRATCH_REG1 read proof remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C26 alive:'
 '[QUAL] K14.C26 final-k14-mmio-allowlist runtime reached intentional post-userspace halt'
 '[K14DONE] K14 native Radeon foundation completion gate reached; broader driver bring-up moves to K15'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C26 final reviewed GFX12 MMIO allowlist/read-only completion gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C26 final-k14-mmio-allowlist runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C26 final-k14-mmio-allowlist runtime qualification PASSED.'
