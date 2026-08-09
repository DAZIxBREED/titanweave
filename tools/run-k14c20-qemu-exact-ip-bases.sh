#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/tools/build.sh"
ESP="$ROOT/build/esp"
LOG="${K14C20_SERIAL_LOG:-$ROOT/build/k14c20-serial.log}"
NVME_IMAGE="${K14C20_NVME_IMAGE:-$ROOT/build/k14c20-nvme-test.img}"
NVME_SIZE="${K14C20_NVME_SIZE:-256M}"
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
    OVMF_CODE="$(find_first /usr/share/OVMF/OVMF_CODE.fd /usr/share/edk2/ovmf/OVMF_CODE.fd /usr/share/edk2/x64/OVMF_CODE.fd)" || { echo 'Could not locate OVMF_CODE.fd. Install edk2-ovmf.' >&2; exit 1; }
fi
if [[ -z "$OVMF_VARS" ]]; then
    OVMF_VARS="$(find_first /usr/share/OVMF/OVMF_VARS.fd /usr/share/edk2/ovmf/OVMF_VARS.fd /usr/share/edk2/x64/OVMF_VARS.fd)" || { echo 'Could not locate OVMF_VARS.fd. Install edk2-ovmf.' >&2; exit 1; }
fi

mkdir -p "$ROOT/build"
OVMF_VARS_RUN="$ROOT/build/OVMF_VARS-K14C20.fd"
cp "$OVMF_VARS" "$OVMF_VARS_RUN"
if [[ ! -f "$NVME_IMAGE" ]]; then truncate -s "$NVME_SIZE" "$NVME_IMAGE"; fi
: > "$LOG"

IOMMU_ARGS=()
if [[ "${K13_IOMMU:-1}" == '1' ]]; then IOMMU_ARGS=(-device intel-iommu,intremap=on,caching-mode=on); fi
DISPLAY_ARGS=()
case "$DISPLAY_BACKEND" in
    none) DISPLAY_ARGS=(-display none) ;;
    gtk)  DISPLAY_ARGS=(-display gtk,gl=off) ;;
    sdl)  DISPLAY_ARGS=(-display sdl,gl=off) ;;
    *)    DISPLAY_ARGS=(-display "$DISPLAY_BACKEND") ;;
esac

QEMU_ARGS=(
    -machine q35 -cpu max -m "${K13_MEMORY:-1024M}" -smp "${K13_CPUS:-4}"
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE"
    -drive "if=pflash,format=raw,file=$OVMF_VARS_RUN"
    -drive "format=raw,file=fat:rw:$ESP"
    -drive "if=none,id=titanfs,format=raw,readonly=on,file=$ROOT/build/titanfs-virtio.img"
    -device virtio-blk-pci,drive=titanfs,disable-modern=on
    -drive "if=none,id=k14c20nvme,format=raw,file=$NVME_IMAGE"
    -device nvme,drive=k14c20nvme,serial=TWK14C20NVME001
    -device qemu-xhci,id=xhci -device usb-kbd,bus=xhci.0 -device usb-tablet,bus=xhci.0
    -vga std
    -device virtio-gpu-pci,id=twk14gpu0,max_outputs=2,iommu_platform=off
    -device virtio-gpu-pci,id=twk14gpu1,max_outputs=1,iommu_platform=off
    -device edu,id=twk14c20iommutest,dma_mask=0xffffffffffffffff
    -monitor none -no-reboot -d guest_errors
)
QEMU_ARGS+=("${DISPLAY_ARGS[@]}")
QEMU_ARGS+=("${IOMMU_ARGS[@]}")

cat <<MSG
Titanweave K14.C20 exact-IP-base QEMU test
  serial log     : $LOG
  display        : $DISPLAY_BACKEND
  K12 scanout    : stdvga/GOP fallback retained
  active GPU     : VirtIO-GPU modern PCI, K13 qualified fallback
  standby GPU    : second VirtIO-GPU topology candidate
  DMA test       : QEMU EDU PCI endpoint (1234:11e8)
  xHCI/HID       : enabled
  VT-d           : ${K13_IOMMU:-1}

QEMU does not emulate a native Radeon. K14.C20 consumes only a C19 checksum-qualified AMD discovery snapshot and walks the packed die/IP records using source-backed GC/SDMA hardware IDs. On QEMU there is no physical Radeon, so C20 must take the explicit no-Radeon deferred path while its bounded parser/self-tests and userspace ABI qualify. On supported bare metal C20 may resolve exact GC/SDMA dword bases, but it performs no MMIO write, firmware upload, command submission, BAR resize, or Radeon bus-master enable. VirtIO-GPU/GOP remain the qualified fallback.
The kernel intentionally halts after native-service qualification.
MSG

set +e
qemu-system-x86_64 "${QEMU_ARGS[@]}" -serial stdio 2>&1 | tee "$LOG"
status=${PIPESTATUS[0]}
set -e

echo "QEMU exit status: $status"
checker_status=0
"$ROOT/tools/check-k14c20-serial-log.sh" "$LOG" || checker_status=$?
if (( status != 0 )); then exit "$status"; fi
exit "$checker_status"
