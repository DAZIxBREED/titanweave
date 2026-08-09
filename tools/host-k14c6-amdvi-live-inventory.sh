#!/usr/bin/env bash
set -euo pipefail
echo "Titanweave K14.C6 AMD-Vi/Radeon host inventory (read-only)"
echo "=== AMD display/audio PCI functions ==="
lspci -nnk | grep -A3 -Ei 'VGA|Display|Audio.*AMD|AMD.*Audio' || true
echo "=== IOMMU groups ==="
for d in /sys/kernel/iommu_groups/*/devices/*; do [[ -e "$d" ]] || continue; b=$(basename "$d"); if lspci -nns "$b" 2>/dev/null | grep -qi '1002:'; then echo "$d -> $(lspci -nns "$b")"; fi; done
echo "=== AMD-Vi/IVRS kernel hints ==="
dmesg 2>/dev/null | grep -Ei 'AMD-Vi|IVRS|IOMMU' | tail -80 || true
