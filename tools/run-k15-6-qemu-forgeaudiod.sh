#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/tools/build.sh"
ESP="$ROOT/build/esp"
LOG="${K15_6_SERIAL_LOG:-$ROOT/build/k15-6-serial.log}"
NVME_IMAGE="${K15_6_NVME_IMAGE:-$ROOT/build/k15-6-nvme-test.img}"
NVME_SIZE="${K15_6_NVME_SIZE:-256M}"
DISPLAY_BACKEND="${K15_DISPLAY:-none}"

find_first() {
    local candidate
    for candidate in "$@"; do
        if [[ -f "$candidate" ]]; then printf '%s\n' "$candidate"; return 0; fi
    done
    return 1
}

OVMF_CODE="${OVMF_CODE:-}"
OVMF_VARS="${OVMF_VARS:-}"
if [[ -z "$OVMF_CODE" ]]; then
    OVMF_CODE="$(find_first /usr/share/OVMF/OVMF_CODE.fd /usr/share/edk2/ovmf/OVMF_CODE.fd /usr/share/edk2/x64/OVMF_CODE.fd)" || {
        echo 'Could not locate OVMF_CODE.fd. Install edk2-ovmf.' >&2; exit 1;
    }
fi
if [[ -z "$OVMF_VARS" ]]; then
    OVMF_VARS="$(find_first /usr/share/OVMF/OVMF_VARS.fd /usr/share/edk2/ovmf/OVMF_VARS.fd /usr/share/edk2/x64/OVMF_VARS.fd)" || {
        echo 'Could not locate OVMF_VARS.fd. Install edk2-ovmf.' >&2; exit 1;
    }
fi

mkdir -p "$ROOT/build"
OVMF_VARS_RUN="$ROOT/build/OVMF_VARS-K15_6.fd"
cp "$OVMF_VARS" "$OVMF_VARS_RUN"
if [[ ! -f "$NVME_IMAGE" ]]; then truncate -s "$NVME_SIZE" "$NVME_IMAGE"; fi
: > "$LOG"

IOMMU_ARGS=()
if [[ "${K13_IOMMU:-1}" == '1' ]]; then
    # K15.4 data DMA is translated through VT-d. Interrupt remapping remains
    # disabled because Titanweave's current generic MSI primitive programs a
    # direct APIC MSI message; K15.4 does not claim VT-d IRTE support.
    IOMMU_ARGS=(-device intel-iommu,intremap=off,caching-mode=on)
fi

DISPLAY_ARGS=()
case "$DISPLAY_BACKEND" in
    none) DISPLAY_ARGS=(-display none) ;;
    gtk)  DISPLAY_ARGS=(-display gtk,gl=off) ;;
    sdl)  DISPLAY_ARGS=(-display sdl,gl=off) ;;
    *)    DISPLAY_ARGS=(-display "$DISPLAY_BACKEND") ;;
esac

QEMU_ARGS=(
    -machine q35
    -cpu max
    -m "${K13_MEMORY:-1024M}"
    -smp "${K13_CPUS:-4}"
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE"
    -drive "if=pflash,format=raw,file=$OVMF_VARS_RUN"
    -drive "format=raw,file=fat:rw:$ESP"
    -drive "if=none,id=titanfs,format=raw,readonly=on,file=$ROOT/build/titanfs-virtio.img"
    -device virtio-blk-pci,drive=titanfs,disable-modern=on
    -drive "if=none,id=k15nvme,format=raw,file=$NVME_IMAGE"
    -device nvme,drive=k15nvme,serial=TWK15AUDIO001
    -device qemu-xhci,id=xhci
    -device usb-kbd,bus=xhci.0
    -device usb-tablet,bus=xhci.0
    -vga std
    -device virtio-gpu-pci,id=twk14gpu0,max_outputs=2,iommu_platform=off
    -device virtio-gpu-pci,id=twk14gpu1,max_outputs=1,iommu_platform=off
    -device edu,id=twk15iommutest,dma_mask=0xffffffffffffffff
    -audiodev driver=none,id=twk15audio
    -device ich9-intel-hda,id=twk15hda,msi=on,bus=pcie.0,addr=1b.0
    -device hda-duplex,bus=twk15hda.0,cad=0,audiodev=twk15audio
    -monitor none
    -no-reboot
    -d guest_errors
)
QEMU_ARGS+=("${DISPLAY_ARGS[@]}")
QEMU_ARGS+=("${IOMMU_ARGS[@]}")

cat <<MSG
Titanweave K15.6 ForgeAudioD QEMU qualification
  serial log       : $LOG
  display          : $DISPLAY_BACKEND
  HDA controller   : QEMU ICH9 Intel HDA, MSI enabled
  HDA codec        : QEMU hda-duplex, codec address 0
  host audio       : null backend (no host speaker/microphone required)
  data VT-d        : ${K13_IOMMU:-1}
  interrupt remap  : off (direct APIC MSI; no IRTE claim)

K15.6 consumes frozen/qualified K15.5. Frozen K15.4/K15.5 hardware+PCM evidence must remain green first. K15.6 then
launches a real persistent ForgeAudioD userspace service. The daemon must own the real HDA device and both endpoints through ForgeAudio handles, own prepared playback/capture stream objects, two bounded server buffers, a clock/event/fence set, a bounded two-route control generation, prove control-plane recovery by rebuilding an invalid stream, publish ownership that the kernel cross-validates by PID, yield, and later heartbeat from the same persistent process.

K15.7 Lock-Free Audio Transport remains locked until this runtime gate passes.
MSG

TEST_TIMEOUT="${K15_6_TIMEOUT:-150}"
HALT_MARKER='[HALT] BSP halted intentionally'

cleanup_qemu() {
    if [[ -n "${qemu_pid:-}" ]] && kill -0 "$qemu_pid" 2>/dev/null; then
        kill -TERM "$qemu_pid" 2>/dev/null || true
        sleep 0.2
        kill -KILL "$qemu_pid" 2>/dev/null || true
    fi
}
trap cleanup_qemu EXIT INT TERM

set +e
qemu-system-x86_64 "${QEMU_ARGS[@]}" -serial stdio > >(tee "$LOG") 2>&1 &
qemu_pid=$!
set -e

halt_seen=0
timed_out=0
deadline=$((SECONDS + TEST_TIMEOUT))
while kill -0 "$qemu_pid" 2>/dev/null; do
    if grep -Fq "$HALT_MARKER" "$LOG"; then halt_seen=1; break; fi
    if (( SECONDS >= deadline )); then timed_out=1; break; fi
    sleep 0.1
done
if grep -Fq "$HALT_MARKER" "$LOG"; then halt_seen=1; fi

if (( halt_seen )); then
    echo
    echo "Intentional Titanweave HALT detected; terminating QEMU."
    kill -TERM "$qemu_pid" 2>/dev/null || true
elif (( timed_out )); then
    echo
    echo "K15.6 QEMU qualification timed out after ${TEST_TIMEOUT}s." >&2
    kill -TERM "$qemu_pid" 2>/dev/null || true
    sleep 0.5
    kill -KILL "$qemu_pid" 2>/dev/null || true
fi

set +e
wait "$qemu_pid"
status=$?
set -e
qemu_pid=""
trap - EXIT INT TERM

checker_status=0
"$ROOT/tools/check-k15-6-serial-log.sh" "$LOG" || checker_status=$?

if (( halt_seen )); then
    echo "QEMU stopped after intentional kernel halt (raw exit status: $status)"
    exit "$checker_status"
fi

echo "QEMU exit status: $status"
if (( timed_out )); then exit 124; fi
if (( status != 0 )); then exit "$status"; fi
exit "$checker_status"
