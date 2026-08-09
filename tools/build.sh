#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${PROFILE:-debug}"

"$ROOT/tools/validate-source.sh"
"$ROOT/tools/build-userspace.sh"
python3 "$ROOT/tools/make-fat32.py" \
    --userspace "$ROOT/build/userspace" \
    --output "$ROOT/build/TITANFS.IMG"
cp "$ROOT/build/TITANFS.IMG" "$ROOT/build/titanfs-virtio.img"

if [[ "$PROFILE" == "release" ]]; then
    PROFILE_ARGS=(--release)
    PROFILE_DIR=release
else
    PROFILE_ARGS=()
    PROFILE_DIR=debug
fi

cargo build -p titanweave-user-runtime --target x86_64-unknown-none "${PROFILE_ARGS[@]}"

# Seal the kernel rustflags for this invocation. Cargo configuration is hierarchical,
# and array-valued target rustflags from ~/.cargo/config.toml or a parent .cargo
# directory are merged with this repository's .cargo/config.toml. If a developer
# has an older Titanweave target stanza globally, that can duplicate -T/--entry.
# rust-lld treats the same PHDRS linker script twice as a script replay and can
# emit ET_EXEC with e_phnum=0. CARGO_ENCODED_RUSTFLAGS has higher precedence than
# target.*.rustflags, so this gives WeaveCore one deterministic flag set regardless
# of ambient Cargo configuration.
KERNEL_RUSTFLAGS=$'-C\x1flink-arg=-Tkernel/weavecore/linker.ld\x1f-C\x1frelocation-model=static\x1f-C\x1fcode-model=kernel\x1f-C\x1fno-redzone=yes'
CARGO_ENCODED_RUSTFLAGS="$KERNEL_RUSTFLAGS" \
  cargo build -p weavecore --target x86_64-unknown-none "${PROFILE_ARGS[@]}"

KERNEL_ELF="$ROOT/target/x86_64-unknown-none/$PROFILE_DIR/weavecore"
python3 "$ROOT/tools/check-kernel-elf.py" "$KERNEL_ELF"

cargo build -p titanweave-uefi-loader --target x86_64-unknown-uefi "${PROFILE_ARGS[@]}"

ESP="$ROOT/build/esp"
EFI_BOOT="$ESP/EFI/BOOT"
rm -rf "$ESP"
mkdir -p "$EFI_BOOT"

cp "$ROOT/target/x86_64-unknown-uefi/$PROFILE_DIR/titanweave-uefi-loader.efi" \
   "$EFI_BOOT/BOOTX64.EFI"
cp "$ROOT/target/x86_64-unknown-none/$PROFILE_DIR/weavecore" \
   "$ESP/WEAVECORE.ELF"
cp "$ROOT/build/TITANFS.IMG" "$ESP/TITANFS.IMG"

echo "Titanweave K14 ESP staged at: $ESP"
echo "Titanweave K14 FAT32 system volume: $ROOT/build/TITANFS.IMG"
