#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/tools/build.sh"
ESP="$ROOT/build/esp"
LOG="${K13B_SERIAL_LOG:-$ROOT/build/k13b-serial.log}"
NVME_IMAGE="${K13B_NVME_IMAGE:-$ROOT/build/k13b-nvme-test.img}"
NVME_SIZE="${K13B_NVME_SIZE:-256M}"
DISPLAY_BACKEND="${K13_DISPLAY:-gtk}"

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
OVMF_VARS_RUN="$ROOT/build/OVMF_VARS-K13B.fd"
cp "$OVMF_VARS" "$OVMF_VARS_RUN"
if [[ ! -f "$NVME_IMAGE" ]]; then truncate -s "$NVME_SIZE" "$NVME_IMAGE"; fi
: > "$LOG"

IOMMU_ARGS=()
if [[ "${K13_IOMMU:-1}" == '1' ]]; then
    IOMMU_ARGS=(-device intel-iommu,intremap=on,caching-mode=on)
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
    -drive "if=none,id=k13nvme,format=raw,file=$NVME_IMAGE"
    -device nvme,drive=k13nvme,serial=TWK13BNVME001
    -device qemu-xhci,id=xhci
    -device usb-kbd,bus=xhci.0
    -device usb-tablet,bus=xhci.0
    -vga std
    -device virtio-gpu-pci,id=twk13gpu,max_outputs=1,iommu_platform=off
    -monitor none
    -no-reboot
    -d guest_errors
)
QEMU_ARGS+=("${DISPLAY_ARGS[@]}")
QEMU_ARGS+=("${IOMMU_ARGS[@]}")

cat <<MSG
Titanweave K13.B VirtIO-GPU transport QEMU test
  serial log     : $LOG
  display        : $DISPLAY_BACKEND
  K12 scanout    : stdvga/GOP fallback retained
  GPU transport  : VirtIO-GPU modern PCI control/cursor queues
  xHCI/HID       : enabled
  VT-d           : ${K13_IOMMU:-1}

K13.B must retain the K12 stdvga/GOP fallback while bringing the secondary VirtIO-GPU modern transport online.
The transport uses bounded ForgeBus DMA ownership; ACCESS_PLATFORM remains disabled until hardware IOMMU page-table programming is implemented.
The kernel intentionally halts after native-service qualification.
MSG

set +e
qemu-system-x86_64 "${QEMU_ARGS[@]}" -serial stdio 2>&1 | tee "$LOG"
status=${PIPESTATUS[0]}
set -e

echo "QEMU exit status: $status"
"$ROOT/tools/check-k13b-serial-log.sh" "$LOG" || true
exit "$status"
