#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${1:-$ROOT/build/k15-4-serial.log}"
[[ -f "$LOG" ]] || { echo "K15.4 serial log not found: $LOG" >&2; exit 1; }

failed=0
if ! "$ROOT/tools/check-k15-3-serial-log.sh" "$LOG"; then
    echo 'FAIL  inherited K15.3 runtime qualification regressed' >&2
    failed=1
fi

required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[K15OK] K15.1 ForgeAudio real-time execution foundation qualified:'
 '[K15OK] K15.2 ForgeAudio kernel ABI qualified:'
 '[K15OK] K15.3 ForgeAudio audio DMA transport qualified:'
 '[K15HDA] controller:'
 '[IOMP] temporary translated coexistence:'
 '[IOMA] temporary translated device domain armed:'
 '[K15HDA] command+codec:'
 '[K15HDA] DMA+IRQ:'
 '[IOMV] temporary translated device domain revoked:'
 '[K15HDA] ForgeAudio registry:'
 '[K15OK] K15.4 ForgeAudio real HDA hardware backend qualified:'
 '[K15HR] ForgeAudio HDA ready:'
 '[K15CO] HDA/GPU coexistence:'
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

controller=$(grep -F '[K15HDA] controller:' "$LOG" | tail -n1 || true)
for token in 'reset=true' 'inputs=' 'outputs=' 'codec_mask='; do
    if [[ "$controller" == *"$token"* ]]; then echo "PASS  controller $token"; else echo "FAIL  controller missing $token" >&2; failed=1; fi
done
if [[ "$controller" == *'codec_mask=0x0000'* ]]; then echo 'FAIL  HDA codec mask is zero' >&2; failed=1; fi

armed=$(grep -F '[IOMA] temporary translated device domain armed:' "$LOG" | tail -n1 || true)
for token in 'requester=0x' 'domain=5444' 'target_bus_master=true'; do
    if [[ "$armed" == *"$token"* ]]; then echo "PASS  translated-domain $token"; else echo "FAIL  translated-domain missing $token" >&2; failed=1; fi
done

coexist=$(grep -F '[IOMP] temporary translated coexistence:' "$LOG" | tail -n1 || true)
for token in 'passthrough_busmasters=' 'unrelated_busmasters_untouched=true'; do
    if [[ "$coexist" == *"$token"* ]]; then echo "PASS  coexistence $token"; else echo "FAIL  coexistence missing $token" >&2; failed=1; fi
done

codec=$(grep -F '[K15HDA] command+codec:' "$LOG" | tail -n1 || true)
for token in 'CORB=true' 'RIRB=true' 'vendor=0x' 'widgets=' 'playback_converter=' 'capture_converter='; do
    if [[ "$codec" == *"$token"* ]]; then echo "PASS  codec $token"; else echo "FAIL  codec evidence missing $token" >&2; failed=1; fi
done

irq=$(grep -F '[K15HDA] DMA+IRQ:' "$LOG" | tail -n1 || true)
for token in 'translated=true' 'BDL=true' 'MSI=true' 'playback_periods=2' 'capture_periods=2' 'playback_frames=2048' 'capture_frames=2048' 'capture_memory_changed=true' 'bus_master_after=false'; do
    if [[ "$irq" == *"$token"* ]]; then echo "PASS  DMA/IRQ $token"; else echo "FAIL  DMA/IRQ missing $token" >&2; failed=1; fi
done
stream_irqs=$(sed -n 's/.*stream_irqs=\([0-9][0-9]*\).*/\1/p' <<<"$irq" | tail -n1)
if [[ -n "$stream_irqs" ]] && (( stream_irqs >= 4 )); then
    echo "PASS  real HDA stream interrupts=$stream_irqs"
else
    echo "FAIL  expected at least four real HDA stream interrupts, got ${stream_irqs:-none}" >&2
    failed=1
fi

revoked=$(grep -F '[IOMV] temporary translated device domain revoked:' "$LOG" | tail -n1 || true)
for token in 'target_bus_master=false' 'translation=false' 'peers_preserved=true'; do
    if [[ "$revoked" == *"$token"* ]]; then echo "PASS  revoke $token"; else echo "FAIL  revoke missing $token" >&2; failed=1; fi
done

registry=$(grep -F '[K15HDA] ForgeAudio registry:' "$LOG" | tail -n1 || true)
for token in 'device=true' 'endpoints=2' 'playback=true' 'capture=true' 'backend=HDA' 'placeholder=false'; do
    if [[ "$registry" == *"$token"* ]]; then echo "PASS  registry $token"; else echo "FAIL  registry missing $token" >&2; failed=1; fi
done

ok=$(grep -F '[K15OK] K15.4 ForgeAudio real HDA hardware backend qualified:' "$LOG" | tail -n1 || true)
for token in 'pci=true' 'reset=true' 'CORB=true' 'RIRB=true' 'codecs=true' 'widgets=true' 'BDL=true' 'translated_dma=true' 'MSI=true' 'irq=true' 'playback=true' 'capture=true' 'registry=true' 'fake_hw=false' 'physical_silicon=false'; do
    if [[ "$ok" == *"$token"* ]]; then echo "PASS  K15.4 $token"; else echo "FAIL  K15.4 final proof missing $token" >&2; failed=1; fi
done

ready=$(grep -F '[K15HR] ForgeAudio HDA ready:' "$LOG" | tail -n1 || true)
for token in 'version=1' 'CORB=true' 'RIRB=true' 'BDL=true' 'translated_dma=true' 'MSI=true' 'playback_periods=2' 'capture_periods=2' 'endpoints=2' 'physical_silicon=false'; do
    if [[ "$ready" == *"$token"* ]]; then echo "PASS  K15.4 ready $token"; else echo "FAIL  K15.4 ready line missing $token" >&2; failed=1; fi
done

gpu_coexist=$(grep -F '[K15CO] HDA/GPU coexistence:' "$LOG" | tail -n1 || true)
for token in 'virtio_transport=true' 'driver_ok=true' 'bus_master=true' 'presentation_rearmed=true'; do
    if [[ "$gpu_coexist" == *"$token"* ]]; then echo "PASS  HDA/GPU $token"; else echo "FAIL  HDA/GPU coexistence missing $token" >&2; failed=1; fi
done

if grep -Fq '[FAIL]' "$LOG"; then
    echo 'FAIL  serial log contains [FAIL]' >&2
    grep -F '[FAIL]' "$LOG" >&2 || true
    failed=1
fi

if (( failed )); then
    echo 'Titanweave K15.4 ForgeAudio real HDA hardware backend runtime qualification FAILED.' >&2
    exit 1
fi

echo 'Titanweave K15.4 ForgeAudio real HDA hardware backend runtime qualification PASSED.'
