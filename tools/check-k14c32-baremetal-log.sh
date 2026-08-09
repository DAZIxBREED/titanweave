#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-}"
[[ -n "$LOG" && -f "$LOG" ]] || { echo 'Usage: check-k14c32-baremetal-log.sh <physical-Radeon-serial.log>' >&2; exit 2; }
# This checker is deliberately stricter than QEMU. It cannot be satisfied by
# fallback/reference evidence and exists to preserve physical-silicon truth.
required=(
 '[C32OK] K14.C32 production/stability + final K14:'
 'amd_present=true'
 'physical_stress=true'
 '[K14DONE] Titanweave native Radeon driver foundation operational'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do if grep -Fq "$marker" "$LOG"; then echo "BARE-METAL PASS  $marker"; else echo "BARE-METAL FAIL  $marker" >&2; failed=1; fi; done
if grep -Fq 'physical_stress_qualified=false' "$LOG"; then echo 'BARE-METAL FAIL  physical stress is explicitly unqualified.' >&2; failed=1; fi
if ((failed)); then echo 'Titanweave K14.C32 BARE-METAL Radeon evidence check FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C32 BARE-METAL Radeon evidence check PASSED.'
