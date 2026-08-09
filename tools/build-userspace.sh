#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/build/userspace"
mkdir -p "$OUT"

for app in init logd console displayd archive trustd driverd shell; do
    upper="$(printf '%s' "$app" | tr '[:lower:]' '[:upper:]')"
    if [[ "$upper" == "TRUSTD" ]]; then
        upper="TRUSTD"
    fi
    if [[ "$upper" == "CONSOLE" ]]; then
        upper="CONSOL"
    fi
    clang -c -m64 -ffreestanding -fno-pic -nostdlib \
        -I"$ROOT" \
        "$ROOT/userspace/$app/$app.S" \
        -o "$OUT/$app.o"
    ld.lld -nostdlib -z max-page-size=0x1000 \
        -T "$ROOT/userspace/user.ld" \
        "$OUT/$app.o" \
        -o "$OUT/$upper.ELF"
done

echo "Titanweave K14 user ELF programs built in: $OUT"
