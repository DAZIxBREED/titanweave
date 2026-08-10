#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${1:-$ROOT/build/k15-8-serial.log}"

if [[ ! -f "$LOG" ]]; then
    echo "K15.8 serial log not found: $LOG" >&2
    exit 1
fi

if ! "$ROOT/tools/check-k15-7-serial-log.sh" "$LOG" >/dev/null; then
    echo 'FAIL  inherited K15.1-K15.7 ForgeAudio qualification did not remain green' >&2
    "$ROOT/tools/check-k15-7-serial-log.sh" "$LOG" || true
    exit 1
fi

echo 'PASS  inherited K15.1-K15.7 ForgeAudio qualification retained'
failed=0
required=(
    '[K15GR] graph compiled:'
    '[K15GR] node execution:'
    '[K15GR] PCM verified:'
    '[K15GR] ForgeAudio graph proof complete:'
    '[K15OK] K15.8 ForgeAudio Graph Engine qualified:'
    '[K15GR] ForgeAudio Graph Engine ready:'
    '[USER] [forgeaudiod] K15.8 graph engine ready:'
    '[C32OK] K14.C32 production/stability + final K14:'
    '[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt'
    '[HALT] BSP halted intentionally'
)
for marker in "${required[@]}"; do
    if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done

compiled=$(grep -F '[K15GR] graph compiled:' "$LOG" | tail -n1 || true)
for token in 'generation=1' 'nodes=8' 'edges=8' 'order=8' 'bounded=true' 'cycle_free=true' 'topology_mutation_rt=false'; do
    if [[ "$compiled" == *"$token"* ]]; then echo "PASS  graph $token"; else echo "FAIL  graph compile missing $token" >&2; failed=1; fi
done

nodes=$(grep -F '[K15GR] node execution:' "$LOG" | tail -n1 || true)
for token in 'Input=4' 'Output=4' 'Gain=4' 'Mixer=4' 'Splitter=4' 'ChannelMapper=4' 'FormatConverter=4' 'Meter=4'; do
    if [[ "$nodes" == *"$token"* ]]; then echo "PASS  node $token"; else echo "FAIL  node execution missing $token" >&2; failed=1; fi
done

pcm=$(grep -F '[K15GR] PCM verified:' "$LOG" | tail -n1 || true)
for token in 'blocks=4' 'frames=1024' 'channels=2' 'format=S16' 'output_verified=true' 'format_roundtrips=4' 'meter_peak=400' 'meter_sum_abs=204800'; do
    if [[ "$pcm" == *"$token"* ]]; then echo "PASS  PCM $token"; else echo "FAIL  PCM proof missing $token" >&2; failed=1; fi
done

proof=$(grep -F '[K15GR] ForgeAudio graph proof complete:' "$LOG" | tail -n1 || true)
for token in 'version=1' 'generation=1' 'nodes=8' 'edges=8' 'Input=true' 'Output=true' 'Gain=true' 'Mixer=true' 'Splitter=true' 'ChannelMapper=true' 'FormatConverter=true' 'Meter=true' 'allocation_free=true' 'runtime_locks=0' 'deterministic_order=true'; do
    if [[ "$proof" == *"$token"* ]]; then echo "PASS  proof $token"; else echo "FAIL  graph proof missing $token" >&2; failed=1; fi
done

ok=$(grep -F '[K15OK] K15.8 ForgeAudio Graph Engine qualified:' "$LOG" | tail -n1 || true)
for token in 'version=1' 'generation=1' 'nodes=8' 'edges=8' 'blocks=4' 'frames=1024' 'Input=true' 'Output=true' 'Gain=true' 'Mixer=true' 'Splitter=true' 'ChannelMapper=true' 'FormatConverter=true' 'Meter=true' 'allocation_free=true' 'bounded=true' 'deterministic_order=true'; do
    if [[ "$ok" == *"$token"* ]]; then echo "PASS  K15.8 $token"; else echo "FAIL  K15.8 final proof missing $token" >&2; failed=1; fi
done

ready=$(grep -F '[K15GR] ForgeAudio Graph Engine ready:' "$LOG" | tail -n1 || true)
for token in 'max_nodes=16' 'max_inputs=4' 'runtime_locks=0' 'topology_compile_rt=false' 'sample_accurate_switching=false' 'resampling=false'; do
    if [[ "$ready" == *"$token"* ]]; then echo "PASS  ready $token"; else echo "FAIL  graph ready missing $token" >&2; failed=1; fi
done

if grep -Fq '[FAIL]' "$LOG"; then
    echo 'FAIL  serial log contains [FAIL]' >&2
    grep -F '[FAIL]' "$LOG" >&2 || true
    failed=1
fi

if (( failed )); then
    echo 'Titanweave K15.8 ForgeAudio Graph Engine runtime qualification FAILED.' >&2
    exit 1
fi

echo 'Titanweave K15.8 ForgeAudio Graph Engine runtime qualification PASSED.'
