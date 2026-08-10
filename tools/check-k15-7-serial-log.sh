#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${1:-$ROOT/build/k15-7-serial.log}"

if [[ ! -f "$LOG" ]]; then
    echo "K15.7 serial log not found: $LOG" >&2
    exit 1
fi

if ! TITANWEAVE_ALLOW_LATER_GATE_FAILURES=1 "$ROOT/tools/check-k15-6-serial-log.sh" "$LOG" >/dev/null; then
    echo 'FAIL  inherited K15.1-K15.6 ForgeAudio qualification did not remain green' >&2
    TITANWEAVE_ALLOW_LATER_GATE_FAILURES=1 "$ROOT/tools/check-k15-6-serial-log.sh" "$LOG" || true
    exit 1
fi

echo 'PASS  inherited K15.1-K15.6 ForgeAudio qualification retained'
failed=0
required=(
    '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
    '[K15OK] K15.6 ForgeAudioD userspace audio server qualified:'
    '[ELF ] Loaded audio-client:'
    '[K15LF] transport attached:'
    '[USER] [audioclient] K15.7 attached:'
    '[USER] [audioclient] K15.7 command queue:'
    '[USER] [forgeaudiod] K15.7 command queues:'
    '[USER] [audioclient] K15.7 audio rings:'
    '[USER] [forgeaudiod] K15.7 audio transport:'
    '[USER] [audioclient] K15.7 exiting normally to trigger dead-client isolation'
    '[K15LF] dead client isolated:'
    '[USER] [forgeaudiod] K15.7 dead-client isolation:'
    '[K15D] ForgeAudioD heartbeat: pid='
    '[USER] [forgeaudiod] K15.7 post-isolation heartbeat:'
    '[K15LF] required lock-free transport + dead-client isolation milestones complete; userspace qualification may close'
    '[K15OK] K15.7 ForgeAudio lock-free transport qualified:'
    '[K15LR] ForgeAudio lock-free transport ready:'
    '[USER] [forgeaudiod] K15.7 lock-free transport ready:'
    '[C32OK] K14.C32 production/stability + final K14:'
    '[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt'
    '[HALT] BSP halted intentionally'
)
for marker in "${required[@]}"; do
    if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done

attach=$(grep -F '[K15LF] transport attached:' "$LOG" | tail -n1 || true)
for token in 'session=1' 'generation=1' 'slots=4' 'block_bytes=1024' 'command_depth=16' 'lock_free=true'; do
    if [[ "$attach" == *"$token"* ]]; then echo "PASS  attach $token"; else echo "FAIL  attach missing $token" >&2; failed=1; fi
done

client_cmd=$(grep -F '[USER] [audioclient] K15.7 command queue:' "$LOG" | tail -n1 || true)
for token in 'depth=16' 'roundtrips=16' 'full=true' 'empty=true' 'sequence=true'; do
    if [[ "$client_cmd" == *"$token"* ]]; then echo "PASS  command $token"; else echo "FAIL  command missing $token" >&2; failed=1; fi
done

client_audio=$(grep -F '[USER] [audioclient] K15.7 audio rings:' "$LOG" | tail -n1 || true)
for token in 'playback_blocks=12' 'capture_blocks=12' 'wraps=3' 'full=true' 'empty=true' 'data_verified=true'; do
    if [[ "$client_audio" == *"$token"* ]]; then echo "PASS  audio $token"; else echo "FAIL  audio missing $token" >&2; failed=1; fi
done

dead=$(grep -F '[K15LF] dead client isolated:' "$LOG" | tail -n1 || true)
for token in 'session=1' 'old_generation=1' 'new_generation=2' 'rings_reset=true' 'server_alive=true'; do
    if [[ "$dead" == *"$token"* ]]; then echo "PASS  dead-client $token"; else echo "FAIL  dead-client missing $token" >&2; failed=1; fi
done

server_dead=$(grep -F '[USER] [forgeaudiod] K15.7 dead-client isolation:' "$LOG" | tail -n1 || true)
for token in 'old_generation=1' 'new_generation=2' 'stale_rejected=true' 'reaped=true' 'server_persistent=true'; do
    if [[ "$server_dead" == *"$token"* ]]; then echo "PASS  isolation $token"; else echo "FAIL  isolation missing $token" >&2; failed=1; fi
done

heartbeat=$(grep -F '[USER] [forgeaudiod] K15.7 post-isolation heartbeat:' "$LOG" | tail -n1 || true)
for token in 'sequence=2' 'server_alive=true'; do
    if [[ "$heartbeat" == *"$token"* ]]; then echo "PASS  heartbeat $token"; else echo "FAIL  heartbeat missing $token" >&2; failed=1; fi
done

ok=$(grep -F '[K15OK] K15.7 ForgeAudio lock-free transport qualified:' "$LOG" | tail -n1 || true)
for token in 'playback_blocks=12' 'capture_blocks=12' 'command_roundtrips=16' 'playback_wraps=3' 'capture_wraps=3' 'command_wraps=2' 'stale_rejections=1' 'dead_clients=1' 'generation_advances=1' 'lock_free=true' 'bounded=true' 'dead_client_isolation=true'; do
    if [[ "$ok" == *"$token"* ]]; then echo "PASS  K15.7 $token"; else echo "FAIL  K15.7 final proof missing $token" >&2; failed=1; fi
done

ready=$(grep -F '[K15LR] ForgeAudio lock-free transport ready:' "$LOG" | tail -n1 || true)
for token in 'version=1' 'block_bytes=1024' 'ring_slots=4' 'command_depth=16' 'SPSC=true' 'atomics=true' 'allocation_free=true' 'server_persistent=true'; do
    if [[ "$ready" == *"$token"* ]]; then echo "PASS  ready $token"; else echo "FAIL  ready missing $token" >&2; failed=1; fi
done

if grep -Fq '[USER] [audioclient] no ForgeAudioD transport available;' "$LOG"; then
    echo 'FAIL  K15.7 HDA qualification unexpectedly used dormant audio-client path' >&2
    failed=1
fi
if grep -Fq '[FAIL]' "$LOG"; then
    echo 'FAIL  serial log contains [FAIL]' >&2
    grep -F '[FAIL]' "$LOG" >&2 || true
    failed=1
fi

if (( failed )); then
    echo 'Titanweave K15.7 ForgeAudio lock-free transport runtime qualification FAILED.' >&2
    exit 1
fi

echo 'Titanweave K15.7 ForgeAudio lock-free transport runtime qualification PASSED.'
