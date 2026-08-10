#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${1:-$ROOT/build/k15-2-serial.log}"
[[ -f "$LOG" ]] || { echo "K15.2 serial log not found: $LOG" >&2; exit 1; }

failed=0
if ! "$ROOT/tools/check-k15-1-serial-log.sh" "$LOG"; then
    echo 'FAIL  inherited K15.1 runtime qualification regressed' >&2
    failed=1
fi

required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[K15RT] ForgeAudio RT qualification start:'
 '[K15OK] K15.1 ForgeAudio real-time execution foundation qualified:'
 '[K15RD] ForgeAudio RT ready:'
 '[K15ABI] ForgeAudio ABI v1 online:'
 '[K15ABI] hardware registry honest: devices=0 qemu_deferred=true fake_devices=false'
 '[K15ABI] stream lifecycle state machine qualified: illegal_start_rejected=true recover=true'
 '[K15ABI] real bounded buffer qualified:'
 '[K15ABI] monotonic audio clock qualified:'
 '[K15ABI] bounded event queue qualified:'
 '[K15ABI] monotonic fence qualified:'
 '[K15OK] K15.2 ForgeAudio kernel ABI qualified: ABI=v1 device+endpoint+stream+buffer+clock+event+fence lifecycle real bounded=true fake_devices=false'
 '[K15ARD] ForgeAudio ABI ready: version=1'
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

ard_line=$(grep -F '[K15ARD] ForgeAudio ABI ready:' "$LOG" | tail -n1 || true)
for token in 'version=1' 'real_devices=0' 'fake_devices=false'; do
    if [[ "$ard_line" == *"$token"* ]]; then
        echo "PASS  K15.2 ABI ready $token"
    else
        echo "FAIL  K15.2 ABI ready missing $token" >&2
        failed=1
    fi
done

buffer_line=$(grep -F '[K15ABI] real bounded buffer qualified:' "$LOG" | tail -n1 || true)
for token in 'bytes=256' 'stride=4' 'frames=64' 'sequence=1'; do
    if [[ "$buffer_line" == *"$token"* ]]; then
        echo "PASS  buffer $token"
    else
        echo "FAIL  K15.2 buffer proof missing $token" >&2
        failed=1
    fi
done

fence_line=$(grep -F '[K15ABI] monotonic fence qualified:' "$LOG" | tail -n1 || true)
for token in 'target=4' 'completed=4'; do
    if [[ "$fence_line" == *"$token"* ]]; then
        echo "PASS  fence $token"
    else
        echo "FAIL  K15.2 fence proof missing $token" >&2
        failed=1
    fi
done

if grep -Fq '[FAIL]' "$LOG"; then
    echo 'FAIL  serial log contains [FAIL]' >&2
    grep -F '[FAIL]' "$LOG" >&2 || true
    failed=1
fi

if (( failed )); then
    echo 'Titanweave K15.2 ForgeAudio kernel ABI runtime qualification FAILED.' >&2
    exit 1
fi

echo 'Titanweave K15.2 ForgeAudio kernel ABI runtime qualification PASSED.'
