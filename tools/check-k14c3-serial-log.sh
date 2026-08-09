#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c3-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C3 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[IOMR] hardware translation qualification: backend=IntelVtd translated=true'
 '[C2NF] K14.C2 native persistent-domain/AMD bring-up ready:'
 '[C3IP] AMD IP bring-up graph:'
 '[C3FW] Radeon firmware manifest contract:'
 '[C3MM] Radeon aperture policy:'
 '[C3HW] physical Radeon bring-up:'
 '[C3RD] K14.C3 Radeon bare-metal staging ready:'
 '[C3NF] K14.C3 Radeon bare-metal staging ready:'
 '[USER] [displayd] K14.C3 Radeon bare-metal staging contract online'
 '[USER] [displayd] K14.C3 no physical Radeon in QEMU; native MMIO/firmware/submit remain fenced'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C3 alive:'
 '[QUAL] K14.C3 Radeon staging runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for m in "${required[@]}"; do if grep -Fq "$m" "$LOG"; then printf 'PASS  %s\n' "$m"; else printf 'MISS  %s\n' "$m"; failed=1; fi; done
if ! grep -Eq '\[C3RD\].*amd_present=false.*actual_domain=false.*firmware_upload=false.*command_submit=false.*bus_master=false.*fallback=true' "$LOG"; then
  echo 'FAIL  QEMU Radeon staging safety boundary was not preserved' >&2; failed=1
fi
if grep -Fq '[FAIL] K14.C3 Radeon bare-metal staging failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C3 Radeon staging/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C3 Radeon staging/runtime qualification PASSED.'
