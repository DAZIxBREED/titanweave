#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
missing=0
check(){ if command -v "$1" >/dev/null 2>&1; then printf '[OK]   %-24s %s\n' "$1" "$(command -v "$1")"; else printf '[MISS] %-24s %s\n' "$1" "$2"; missing=1; fi; }
check python3 'required for image generation and policy tests'
check clang 'required for assembly and userspace builds'
check ld.lld 'required for userspace linking'
check readelf 'required for ELF validation'
check objdump 'required for trampoline validation'
check cargo 'required for Rust workspace compilation'
check rustc 'required for Rust workspace compilation'
check rustup 'recommended for target/component management'
check qemu-system-x86_64 'required for virtual boot validation'

ovmf=''
for p in /usr/share/OVMF/OVMF_CODE.fd /usr/share/edk2/ovmf/OVMF_CODE.fd /usr/share/edk2/x64/OVMF_CODE.fd; do [[ -f "$p" ]] && ovmf="$p" && break; done
if [[ -n "$ovmf" ]]; then echo "[OK]   OVMF                     $ovmf"; else echo '[MISS] OVMF                     required for UEFI virtual boot'; missing=1; fi

if command -v rustup >/dev/null 2>&1; then
  for target in x86_64-unknown-none x86_64-unknown-uefi; do
    if rustup target list --installed | grep -qx "$target"; then echo "[OK]   Rust target              $target"; else echo "[MISS] Rust target              $target"; missing=1; fi
  done
fi

echo
if ((missing)); then
  echo 'Build host is not ready. On Fedora run: ./tools/setup-fedora.sh'
  exit 2
fi
echo 'Build host is ready for compile and QEMU gates.'
