#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${1:-$ROOT/build/k15-6-serial.log}"

if [[ ! -f "$LOG" ]]; then
    echo "K15.6 serial log not found: $LOG" >&2
    exit 1
fi

if ! TITANWEAVE_ALLOW_LATER_GATE_FAILURES=1 "$ROOT/tools/check-k15-5-serial-log.sh" "$LOG" >/dev/null; then
    echo 'FAIL  inherited K15.1-K15.5 ForgeAudio qualification did not remain green' >&2
    TITANWEAVE_ALLOW_LATER_GATE_FAILURES=1 "$ROOT/tools/check-k15-5-serial-log.sh" "$LOG" || true
    exit 1
fi

echo 'PASS  inherited K15.1-K15.5 ForgeAudio qualification retained'
failed=0
required=(
    '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
    '[K15OK] K15.5 ForgeAudio PCM format engine qualified:'
    '[ELF ] Loaded forgeaudiod:'
    '[K15D] ForgeAudioD registered:'
    '[USER] [forgeaudiod] K15.6 device ownership:'
    '[USER] [forgeaudiod] K15.6 objects:'
    '[USER] [forgeaudiod] K15.6 control plane:'
    '[USER] [forgeaudiod] K15.6 telemetry:'
    '[USER] [forgeaudiod] K15.6 recovery:'
    '[K15D] ForgeAudioD ownership published:'
    '[K15D] ForgeAudioD ownership verified:'
    '[USER] [forgeaudiod] K15.6 ForgeAudioD ready:'
    '[K15D] ForgeAudioD heartbeat:'
    '[USER] [forgeaudiod] K15.6 persistent heartbeat:'
    '[K15D] required ForgeAudioD ready+heartbeat milestones complete; userspace qualification may close'
    '[K15OK] K15.6 ForgeAudioD userspace audio server qualified:'
    '[K15SR] ForgeAudioD ready:'
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

registered=$(grep -F '[K15D] ForgeAudioD registered:' "$LOG" | tail -n1 || true)
for token in 'singleton=true' 'userspace=true'; do
    if [[ "$registered" == *"$token"* ]]; then echo "PASS  registration $token"; else echo "FAIL  registration missing $token" >&2; failed=1; fi
done

ownership=$(grep -F '[K15D] ForgeAudioD ownership verified:' "$LOG" | tail -n1 || true)
for token in 'streams=2' 'playback=1' 'capture=1' 'prepared=2' 'buffers=2' 'clocks=1' 'events=1' 'fences=1' 'routes=2' 'graph_generation=1' 'recovery=true'; do
    if [[ "$ownership" == *"$token"* ]]; then echo "PASS  ownership $token"; else echo "FAIL  ownership missing $token" >&2; failed=1; fi
done

objects=$(grep -F '[USER] [forgeaudiod] K15.6 objects:' "$LOG" | tail -n1 || true)
for token in 'streams=2' 'buffers=2' 'clocks=1' 'events=1' 'fences=1' 'prepared=true'; do
    if [[ "$objects" == *"$token"* ]]; then echo "PASS  objects $token"; else echo "FAIL  objects missing $token" >&2; failed=1; fi
done

control=$(grep -F '[USER] [forgeaudiod] K15.6 control plane:' "$LOG" | tail -n1 || true)
for token in 'routes=2' 'graph_generation=1' 'bounded=true' 'no_mixing=true' 'no_resampling=true'; do
    if [[ "$control" == *"$token"* ]]; then echo "PASS  control $token"; else echo "FAIL  control missing $token" >&2; failed=1; fi
done


telemetry=$(grep -F '[USER] [forgeaudiod] K15.6 telemetry:' "$LOG" | tail -n1 || true)
for token in 'clock=true' 'event_queue=true' 'empty_event=true' 'fence=true' 'target=1' 'completed=0'; do
    if [[ "$telemetry" == *"$token"* ]]; then echo "PASS  telemetry $token"; else echo "FAIL  telemetry missing $token" >&2; failed=1; fi
done

recovery=$(grep -F '[USER] [forgeaudiod] K15.6 recovery:' "$LOG" | tail -n1 || true)
for token in 'invalid_start_rejected=true' 'stream_rebuilt=true' 'recoveries=1'; do
    if [[ "$recovery" == *"$token"* ]]; then echo "PASS  recovery $token"; else echo "FAIL  recovery missing $token" >&2; failed=1; fi
done

ready=$(grep -F '[USER] [forgeaudiod] K15.6 ForgeAudioD ready:' "$LOG" | tail -n1 || true)
for token in 'device=true' 'streams=true' 'routing=true' 'clocks=true' 'buffers=true' 'telemetry=true' 'recovery=true'; do
    if [[ "$ready" == *"$token"* ]]; then echo "PASS  ready $token"; else echo "FAIL  ready missing $token" >&2; failed=1; fi
done

heartbeat=$(grep -F '[K15D] ForgeAudioD heartbeat:' "$LOG" | tail -n1 || true)
if grep -F '[K15D] ForgeAudioD heartbeat:' "$LOG" | grep -Fq 'sequence=1'; then
    echo 'PASS  heartbeat sequence=1 observed'
else
    echo 'FAIL  heartbeat sequence=1 was never observed' >&2
    failed=1
fi
if [[ "$heartbeat" == *'persistent=true'* ]]; then echo 'PASS  heartbeat persistent=true'; else echo 'FAIL  heartbeat missing persistent=true' >&2; failed=1; fi

ok=$(grep -F '[K15OK] K15.6 ForgeAudioD userspace audio server qualified:' "$LOG" | tail -n1 || true)
for token in 'userspace=true' 'singleton=true' 'device_ownership=true' 'streams=true' 'routing=true' 'clocks=true' 'buffers=true' 'telemetry=true' 'recovery=true' 'persistent=true'; do
    if [[ "$ok" == *"$token"* ]]; then echo "PASS  K15.6 $token"; else echo "FAIL  K15.6 final proof missing $token" >&2; failed=1; fi
done

server_ready=$(grep -F '[K15SR] ForgeAudioD ready:' "$LOG" | tail -n1 || true)
for token in 'routes=2' 'graph_generation=1' 'recoveries=1' 'persistent=true'; do
    if [[ "$server_ready" == *"$token"* ]]; then echo "PASS  server-ready $token"; else echo "FAIL  server-ready missing $token" >&2; failed=1; fi
done
heartbeat_value=$(printf '%s\n' "$server_ready" | sed -n 's/.*heartbeat_sequence=\([0-9][0-9]*\).*/\1/p')
if [[ -n "$heartbeat_value" ]] && (( heartbeat_value >= 1 )); then
    echo "PASS  server-ready heartbeat_sequence=$heartbeat_value"
else
    echo 'FAIL  server-ready heartbeat sequence missing/non-monotonic' >&2
    failed=1
fi

if grep -Fq '[USER] [forgeaudiod] no audio hardware registered;' "$LOG"; then
    echo 'FAIL  K15.6 HDA qualification unexpectedly used dormant ForgeAudioD path' >&2
    failed=1
fi
if [[ "${TITANWEAVE_ALLOW_LATER_GATE_FAILURES:-0}" != '1' ]] && grep -Fq '[FAIL]' "$LOG"; then
    echo 'FAIL  serial log contains [FAIL]' >&2
    grep -F '[FAIL]' "$LOG" >&2 || true
    failed=1
fi

if (( failed )); then
    echo 'Titanweave K15.6 ForgeAudioD runtime qualification FAILED.' >&2
    exit 1
fi

echo 'Titanweave K15.6 ForgeAudioD runtime qualification PASSED.'
