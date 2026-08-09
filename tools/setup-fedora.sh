#!/usr/bin/env bash
set -euo pipefail

sudo dnf install -y \
    qemu-system-x86-core \
    edk2-ovmf \
    curl \
    git \
    gcc \
    clang \
    binutils \
    lld \
    python3

if ! command -v rustup >/dev/null 2>&1; then
    echo "Installing Rust through the official rustup installer..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
fi

rustup toolchain install stable --profile minimal
rustup target add x86_64-unknown-uefi x86_64-unknown-none
rustup component add rust-src llvm-tools-preview rustfmt clippy

echo "Fedora host setup complete. Run: ./tools/run-k12-qemu-display.sh"
