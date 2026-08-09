#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/tools/build.sh"
ESP="$ROOT/build/esp"

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
        echo "Could not locate OVMF_CODE.fd. Install Fedora package: edk2-ovmf" >&2
        exit 1
    }
fi

if [[ -z "$OVMF_VARS" ]]; then
    OVMF_VARS="$(find_first \
        /usr/share/OVMF/OVMF_VARS.fd \
        /usr/share/edk2/ovmf/OVMF_VARS.fd \
        /usr/share/edk2/x64/OVMF_VARS.fd)" || {
        echo "Could not locate OVMF_VARS.fd. Install Fedora package: edk2-ovmf" >&2
        exit 1
    }
fi

mkdir -p "$ROOT/build"
OVMF_VARS_RUN="$ROOT/build/OVMF_VARS.fd"
cp "$OVMF_VARS" "$OVMF_VARS_RUN"

echo "Launching Titanweave K11 baseline with four virtual CPUs and a VirtIO system disk. Press Ctrl+C after [HALT]."
exec qemu-system-x86_64 \
    -machine q35 \
    -cpu max \
    -m 512M \
    -smp 4 \
    -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE" \
    -drive "if=pflash,format=raw,file=$OVMF_VARS_RUN" \
    -drive "format=raw,file=fat:rw:$ESP" \
    -drive "if=none,id=titanfs,format=raw,readonly=on,file=$ROOT/build/titanfs-virtio.img" \
    -device virtio-blk-pci,drive=titanfs,disable-modern=on \
    -serial stdio \
    -monitor none \
    -display none \
    -no-reboot \
    -d guest_errors
