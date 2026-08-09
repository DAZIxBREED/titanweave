#!/usr/bin/env bash
set -euo pipefail
echo 'Titanweave K14.C4 host AMD-Vi/Radeon inventory (read-only)'
echo
printf 'Kernel: '; uname -r
printf 'IOMMU kernel args: '; cat /proc/cmdline
printf '\nAMD/IOMMU dmesg hints (may require sudo):\n'
(dmesg 2>/dev/null || true) | grep -Ei 'AMD-Vi|IOMMU|IVRS' | tail -n 40 || true
printf '\nAMD display/audio functions:\n'
if command -v lspci >/dev/null 2>&1; then
  lspci -Dnnk | awk 'BEGIN{RS=""} /Advanced Micro Devices|AMD\/ATI/ && /(VGA compatible controller|Display controller|Audio device)/ {print $0"\n"}'
else
  echo 'lspci not installed'
fi
printf '\nSysfs AMD display candidates:\n'
for d in /sys/bus/pci/devices/*; do
  [[ -r "$d/vendor" && -r "$d/class" ]] || continue
  vendor=$(cat "$d/vendor"); class=$(cat "$d/class")
  [[ "$vendor" == '0x1002' ]] || continue
  [[ "$class" == 0x03* ]] || continue
  bdf=${d##*/}; echo "--- $bdf"
  printf 'device='; cat "$d/device"
  printf 'class='; cat "$d/class"
  [[ -L "$d/driver" ]] && echo "driver=$(basename "$(readlink "$d/driver")")" || echo 'driver=none'
  [[ -L "$d/iommu_group" ]] && echo "iommu_group=$(basename "$(readlink "$d/iommu_group")")" || echo 'iommu_group=none'
  echo 'resources:'; cat "$d/resource"
done
