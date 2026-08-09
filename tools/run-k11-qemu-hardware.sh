#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/tools/build.sh"
ESP="$ROOT/build/esp"
LOG="${K11_SERIAL_LOG:-$ROOT/build/k11-serial.log}"
NVME_IMAGE="${K11_NVME_IMAGE:-$ROOT/build/k11-nvme-test.img}"
NVME_SIZE="${K11_NVME_SIZE:-256M}"

find_first() {
    local candidate
    for candidate in "$@"; do
        if [[ -f "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

OVMF_CODE="${OVMF_CODE:-}"
OVMF_VARS="${OVMF_VARS:-}"
if [[ -z "$OVMF_CODE" ]]; then
    OVMF_CODE="$(find_first \
        /usr/share/OVMF/OVMF_CODE.fd \
        /usr/share/edk2/ovmf/OVMF_CODE.fd \
        /usr/share/edk2/x64/OVMF_CODE.fd)" || {
        echo "Could not locate OVMF_CODE.fd. Install edk2-ovmf." >&2
        exit 1
    }
fi
if [[ -z "$OVMF_VARS" ]]; then
    OVMF_VARS="$(find_first \
        /usr/share/OVMF/OVMF_VARS.fd \
        /usr/share/edk2/ovmf/OVMF_VARS.fd \
        /usr/share/edk2/x64/OVMF_VARS.fd)" || {
        echo "Could not locate OVMF_VARS.fd. Install edk2-ovmf." >&2
        exit 1
    }
fi

mkdir -p "$ROOT/build"
OVMF_VARS_RUN="$ROOT/build/OVMF_VARS-K11.fd"
cp "$OVMF_VARS" "$OVMF_VARS_RUN"
if [[ ! -f "$NVME_IMAGE" ]]; then
    truncate -s "$NVME_SIZE" "$NVME_IMAGE"
fi
: > "$LOG"

IOMMU_ARGS=()
if [[ "${K11_IOMMU:-1}" == "1" ]]; then
    IOMMU_ARGS=(-device intel-iommu,intremap=on,caching-mode=on)
fi

QEMU_ARGS=(
    -machine q35
    -cpu max
    -m "${K11_MEMORY:-1024M}"
    -smp "${K11_CPUS:-4}"
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE"
    -drive "if=pflash,format=raw,file=$OVMF_VARS_RUN"
    -drive "format=raw,file=fat:rw:$ESP"
    -drive "if=none,id=titanfs,format=raw,readonly=on,file=$ROOT/build/titanfs-virtio.img"
    -device virtio-blk-pci,drive=titanfs,disable-modern=on
    -drive "if=none,id=k11nvme,format=raw,file=$NVME_IMAGE"
    -device nvme,drive=k11nvme,serial=TWK11NVME001
    -device qemu-xhci,id=xhci
    -device usb-kbd,bus=xhci.0
    -device usb-tablet,bus=xhci.0
    -monitor none
    -display none
    -no-reboot
    -d guest_errors
)
QEMU_ARGS+=("${IOMMU_ARGS[@]}")

cat <<MSG
Titanweave K11 hardware-focused QEMU test
  serial log : $LOG
  NVMe image : $NVME_IMAGE
  xHCI/HID   : enabled
  VT-d       : ${K11_IOMMU:-1}

The kernel intentionally halts after the native-service test. Ctrl+C is safe if it does not exit.
MSG

set +e
qemu-system-x86_64 "${QEMU_ARGS[@]}" -serial stdio 2>&1 | tee "$LOG"
status=${PIPESTATUS[0]}
set -e

echo "QEMU exit status: $status"
"$ROOT/tools/check-serial-log.sh" "$LOG" || true
exit "$status"
