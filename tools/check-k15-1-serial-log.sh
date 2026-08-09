#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k15-1-serial.log}"
[[ -f "$LOG" ]] || { echo "K15.1 serial log not found: $LOG" >&2; exit 1; }

failed=0
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[K15RT] ForgeAudio RT qualification start:'
 '[K15RT] policy: fixed-priority+deadline ordering budget_enforced=true PI=true bounded_preempt_guard=true'
 '[K15RT] PI boost observed:'
 '[K15RT] PI waiter acquired mutex after ownership transfer'
 '[K15RT] result:'
 '[K15OK] K15.1 ForgeAudio real-time execution foundation qualified:'
 '[K15RD] ForgeAudio RT ready:'
 '[C32OK] K14.C32 production/stability + final K14:'
 '[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
for marker in "${required[@]}"; do
    if grep -Fq "$marker" "$LOG"; then
        echo "PASS  $marker"
    else
        echo "FAIL  $marker" >&2
        failed=1
    fi
done

jobs=$(grep -Fc '[K15RT] audio job=' "$LOG" || true)
if [[ "$jobs" == "8" ]]; then
    echo 'PASS  [K15RT] exactly 8 periodic audio jobs completed'
else
    echo "FAIL  expected 8 K15.1 audio jobs, saw $jobs" >&2
    failed=1
fi

result_line=$(grep -F '[K15RT] result:' "$LOG" | tail -n1 || true)
for token in 'deadline_misses=0' 'budget_exhaustions=0' 'guard_overruns=0' 'audio_jobs=8'; do
    if [[ "$result_line" == *"$token"* ]]; then
        echo "PASS  $token"
    else
        echo "FAIL  K15.1 result missing $token" >&2
        failed=1
    fi
done

pi_events=$(printf '%s\n' "$result_line" | sed -n 's/.*PI_events=\([0-9][0-9]*\).*/\1/p')
deferrals=$(printf '%s\n' "$result_line" | sed -n 's/.*guard_deferrals=\([0-9][0-9]*\).*/\1/p')
if [[ -n "$pi_events" && "$pi_events" -ge 1 ]]; then
    echo "PASS  priority inheritance events=$pi_events"
else
    echo 'FAIL  no priority inheritance event observed' >&2
    failed=1
fi
if [[ -n "$deferrals" && "$deferrals" -ge 1 ]]; then
    echo "PASS  bounded preemption deferrals=$deferrals"
else
    echo 'FAIL  no bounded preemption deferral observed' >&2
    failed=1
fi

if grep -Fq '[FAIL]' "$LOG"; then
    echo 'FAIL  serial log contains [FAIL]' >&2
    grep -F '[FAIL]' "$LOG" >&2 || true
    failed=1
fi

if (( failed )); then
    echo 'Titanweave K15.1 ForgeAudio RT runtime qualification FAILED.' >&2
    exit 1
fi

echo 'Titanweave K15.1 ForgeAudio RT runtime qualification PASSED.'
