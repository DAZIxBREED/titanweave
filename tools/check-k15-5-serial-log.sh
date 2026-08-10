#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${1:-$ROOT/build/k15-5-serial.log}"

if [[ ! -f "$LOG" ]]; then
    echo "K15.5 serial log not found: $LOG" >&2
    exit 1
fi

# Frozen K15.4 hardware evidence must still pass first.
if ! TITANWEAVE_ALLOW_LATER_GATE_FAILURES=1 "$ROOT/tools/check-k15-4-serial-log.sh" "$LOG" >/dev/null; then
    echo 'FAIL  inherited K15.4 HDA qualification did not remain green' >&2
    TITANWEAVE_ALLOW_LATER_GATE_FAILURES=1 "$ROOT/tools/check-k15-4-serial-log.sh" "$LOG" || true
    exit 1
fi

echo 'PASS  inherited K15.1-K15.4 ForgeAudio qualification retained'
failed=0
required=(
    '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
    '[K15OK] K15.4 ForgeAudio real HDA hardware backend qualified:'
    '[K15PCM] canonical engine:'
    '[K15PCM] HDA format engine:'
    '[K15PCM] layout+channel engine:'
    '[K15PCM] DMA geometry:'
    '[K15PCM] real HDA endpoint binding:'
    '[K15OK] K15.5 ForgeAudio PCM format engine qualified:'
    '[K15PR] ForgeAudio PCM ready:'
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

canonical=$(grep -F '[K15PCM] canonical engine:' "$LOG" | tail -n1 || true)
for token in 'version=1' 'formats=4' 'rates=12' 'max_channels=16' 'interleaved=true' 'planar=true' 'allocation_free=true' 'bounded_frames=2048'; do
    if [[ "$canonical" == *"$token"* ]]; then echo "PASS  canonical $token"; else echo "FAIL  canonical missing $token" >&2; failed=1; fi
done

hda=$(grep -F '[K15PCM] HDA format engine:' "$LOG" | tail -n1 || true)
for token in 'rate_roundtrips=12' 'exact=96000/S24in32/6ch' 'nearest_50000=48000' 'stream_format=0x0835' 'unsupported_rejected=true'; do
    if [[ "$hda" == *"$token"* ]]; then echo "PASS  HDA-format $token"; else echo "FAIL  HDA-format missing $token" >&2; failed=1; fi
done

layout=$(grep -F '[K15PCM] layout+channel engine:' "$LOG" | tail -n1 || true)
for token in 'interleaved_planar=true' 'formats=4' 'channel_mapping=true' 'zero_fill=true' 'no_mixing=true'; do
    if [[ "$layout" == *"$token"* ]]; then echo "PASS  layout $token"; else echo "FAIL  layout missing $token" >&2; failed=1; fi
done

geometry=$(grep -F '[K15PCM] DMA geometry:' "$LOG" | tail -n1 || true)
for token in 'period_frames=256' 'period_bytes=6144' 'periods=4' 'ring_frames=1024' 'ring_bytes=24576' 'within_k15_3=true'; do
    if [[ "$geometry" == *"$token"* ]]; then echo "PASS  geometry $token"; else echo "FAIL  geometry missing $token" >&2; failed=1; fi
done

endpoint=$(grep -F '[K15PCM] real HDA endpoint binding:' "$LOG" | tail -n1 || true)
for token in 'playback=true' 'capture=true' 'rate=48000' 'channels=2' 'format=S16' 'hda_stream=0x0011' 'fake_device=false'; do
    if [[ "$endpoint" == *"$token"* ]]; then echo "PASS  endpoint $token"; else echo "FAIL  endpoint missing $token" >&2; failed=1; fi
done

ok=$(grep -F '[K15OK] K15.5 ForgeAudio PCM format engine qualified:' "$LOG" | tail -n1 || true)
for token in 'canonical=true' 'interleaved_planar=true' 'channel_mapping=true' 'rate_negotiation=true' 'HDA_encode_decode=true' 'period_geometry=true' 'HDA_endpoint=true' 'unsupported_rejected=true' 'fake_device=false'; do
    if [[ "$ok" == *"$token"* ]]; then echo "PASS  K15.5 $token"; else echo "FAIL  K15.5 final proof missing $token" >&2; failed=1; fi
done

ready=$(grep -F '[K15PR] ForgeAudio PCM ready:' "$LOG" | tail -n1 || true)
for token in 'version=1' 'formats=4' 'rates=12' 'HDA_roundtrips=12' 'interleaved_planar=true' 'channel_mapping=true' 'exact=true' 'nearest=true' 'geometry=true' 'HDA_endpoint=true' 'rate=48000' 'channels=2' 'sample_format=S16' 'fake_device=false'; do
    if [[ "$ready" == *"$token"* ]]; then echo "PASS  ready $token"; else echo "FAIL  ready missing $token" >&2; failed=1; fi
done

if [[ "${TITANWEAVE_ALLOW_LATER_GATE_FAILURES:-0}" != '1' ]] && grep -Fq '[FAIL]' "$LOG"; then
    echo 'FAIL  serial log contains [FAIL]' >&2
    grep -F '[FAIL]' "$LOG" >&2 || true
    failed=1
fi

if (( failed )); then
    echo 'Titanweave K15.5 ForgeAudio PCM format engine runtime qualification FAILED.' >&2
    exit 1
fi

echo 'Titanweave K15.5 ForgeAudio PCM format engine runtime qualification PASSED.'
