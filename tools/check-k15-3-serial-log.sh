#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${1:-$ROOT/build/k15-3-serial.log}"
[[ -f "$LOG" ]] || { echo "K15.3 serial log not found: $LOG" >&2; exit 1; }

failed=0
if ! "$ROOT/tools/check-k15-2-serial-log.sh" "$LOG"; then
    echo 'FAIL  inherited K15.2 runtime qualification regressed' >&2
    failed=1
fi

required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[K15OK] K15.1 ForgeAudio real-time execution foundation qualified:'
 '[K15OK] K15.2 ForgeAudio kernel ABI qualified:'
 '[IOMF] K14.B translated DMA qualification ready:'
 '[K15DMA] ForgeAudio DMA transport qualification start:'
 '[K15DMA] real cyclic ring:'
 '[K15DMA] isolation gate:'
 '[K15DMA] cyclic accounting:'
 '[K15DMA] XRUN detection:'
 '[K15OK] K15.3 ForgeAudio audio DMA transport qualified:'
 '[K15DR] ForgeAudio DMA ready:'
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

iommu_line=$(grep -F '[IOMF] K14.B translated DMA qualification ready:' "$LOG" | tail -n1 || true)
if [[ "$iommu_line" == *'translated=true'* ]]; then
    echo 'PASS  K15.3 platform translated DMA proof retained'
else
    echo 'FAIL  K15.3 requires K14.B translated=true on the QEMU qualification target' >&2
    failed=1
fi

isolation_line=$(grep -F '[K15DMA] isolation gate:' "$LOG" | tail -n1 || true)
for token in 'platform_translated=true' 'raw_arm_rejected=true' 'audio_hw_deferred=true' 'fake_dma=false'; do
    if [[ "$isolation_line" == *"$token"* ]]; then
        echo "PASS  isolation $token"
    else
        echo "FAIL  K15.3 isolation proof missing $token" >&2
        failed=1
    fi
done

cyclic_line=$(grep -F '[K15DMA] cyclic accounting:' "$LOG" | tail -n1 || true)
for token in 'completed=12' 'wraps=3' 'frames=1536' 'ownership=true' 'completion_source=transport_core_selftest'; do
    if [[ "$cyclic_line" == *"$token"* ]]; then
        echo "PASS  cyclic $token"
    else
        echo "FAIL  K15.3 cyclic accounting missing $token" >&2
        failed=1
    fi
done

xrun_line=$(grep -F '[K15DMA] XRUN detection:' "$LOG" | tail -n1 || true)
for token in 'playback_underruns=1' 'capture_overruns=1' 'bounded=true'; do
    if [[ "$xrun_line" == *"$token"* ]]; then
        echo "PASS  XRUN $token"
    else
        echo "FAIL  K15.3 XRUN proof missing $token" >&2
        failed=1
    fi
done

ok_line=$(grep -F '[K15OK] K15.3 ForgeAudio audio DMA transport qualified:' "$LOG" | tail -n1 || true)
for token in 'cyclic=true' 'period_completion=true' 'position=true' 'ownership=true' 'iommu_fail_closed=true' 'translated_platform=true' 'xrun=true' 'hardware_audio=false' 'fake_dma=false'; do
    if [[ "$ok_line" == *"$token"* ]]; then
        echo "PASS  K15.3 $token"
    else
        echo "FAIL  K15.3 final proof missing $token" >&2
        failed=1
    fi
done

ready_line=$(grep -F '[K15DR] ForgeAudio DMA ready:' "$LOG" | tail -n1 || true)
for token in 'version=1' 'real_memory=true' 'periods=12' 'wraps=3' 'underruns=1' 'overruns=1' 'translated_platform=true' 'qemu_hda_deferred=true'; do
    if [[ "$ready_line" == *"$token"* ]]; then
        echo "PASS  K15.3 ready $token"
    else
        echo "FAIL  K15.3 ready line missing $token" >&2
        failed=1
    fi
done

# Standalone qualification remains strict. Later gates may reuse this checker
# for inherited evidence while owning the final global [FAIL] scan themselves.
if [[ "${TITANWEAVE_ALLOW_LATER_GATE_FAILURES:-0}" != '1' ]] && grep -Fq '[FAIL]' "$LOG"; then
    echo 'FAIL  serial log contains [FAIL]' >&2
    grep -F '[FAIL]' "$LOG" >&2 || true
    failed=1
fi

if (( failed )); then
    echo 'Titanweave K15.3 ForgeAudio audio DMA transport runtime qualification FAILED.' >&2
    exit 1
fi

echo 'Titanweave K15.3 ForgeAudio audio DMA transport runtime qualification PASSED.'
