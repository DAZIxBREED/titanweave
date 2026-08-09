#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c21-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C21 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C20OK] K14.C20 AMD exact live IP-base gate:'
 '[C21RV] reviewed GFX12 target rebind:'
 '[C21PG] post-discovery identity-write policy:'
 '[C21HW] GFX12 SCRATCH_REG0 identity-write:'
 '[C21RD] K14.C21 reviewed-target rebind ready:'
 '[C21OK] K14.C21 reviewed GFX12 target rebind/identity-write gate:'
 '[USER] [displayd] K14.C21 reviewed GFX12 SCRATCH_REG0 rebind/identity-write gate online'
 '[USER] [displayd] K14.C21 no physical Radeon in QEMU; reviewed GFX12 identity-write remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C21 alive:'
 '[QUAL] K14.C21 reviewed-MMIO-rebind runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C21 reviewed GFX12 target rebind/identity-write gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C21 reviewed-MMIO-rebind runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C21 reviewed-MMIO-rebind runtime qualification PASSED.'
