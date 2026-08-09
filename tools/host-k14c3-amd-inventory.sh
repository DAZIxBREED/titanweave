#!/usr/bin/env bash
set -euo pipefail
echo 'Titanweave K14.C3 host-side AMD GPU inventory (read-only)'
if ! command -v lspci >/dev/null 2>&1; then echo 'lspci is required (pciutils).' >&2; exit 1; fi
mapfile -t BDFS < <(lspci -Dn | awk 'tolower($3) ~ /^1002:/ && tolower($2) ~ /^03/ {print $1}')
if ((${#BDFS[@]}==0)); then echo 'No AMD display-class PCI function found.'; exit 0; fi
for bdf in "${BDFS[@]}"; do
  echo
  echo "AMD display function: $bdf"
  lspci -Dnnks "$bdf" || true
  sys="/sys/bus/pci/devices/${bdf}"
  [[ -e "$sys/resource" ]] && { echo 'BAR resources:'; cat "$sys/resource"; }
  [[ -L "$sys/iommu_group" ]] && echo "IOMMU group: $(basename "$(readlink -f "$sys/iommu_group")")" || echo 'IOMMU group: unavailable'
done
