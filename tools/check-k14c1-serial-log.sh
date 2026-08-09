#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c1-serial.log}"
if [[ ! -f "$LOG" ]]; then echo "K14.C1 serial log not found: $LOG" >&2; exit 1; fi
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[TEST] Breakpoint exception returned successfully'
 '[BUS ] ForgeBus retained'
 '[GPUF] K13 acceleration foundation ready:'
 '[GPUT] K13.B VirtIO-GPU transport ready:'
 '[GPRE] K13.C buffered presentation ready:'
 '[GRDY] K13.D resilience/multi-GPU ready:'
 '[IOMR] hardware translation qualification: backend=IntelVtd translated=true'
 '[IOMF] K14.B translated DMA qualification ready:'
 '[NATF] K14.A native GPU prerequisite foundation ready:'
 '[AMDB] AMD backend foundation self-test:'
 '[NVRM] native VRAM/GTT ownership self-test:'
 '[NSEL] K14.C1 native backend selection:'
 '[NBND] native ForgeBus ownership:'
 '[NDOM] native translated-domain admission:'
 '[NCF ] K14.C1 native binding foundation ready:'
 '[USER] Starting disk-backed K14 native services'
 '[USER] [displayd] K14.C1 native backend ownership foundation online'
 '[USER] [displayd] K14.C1 waiting for bare-metal AMD/Intel/NVIDIA adapter; VirtIO/GOP fallback retained'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C1 alive:'
 '[QUAL] K14.C1 native binding runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then printf 'PASS  %s\n' "$marker"; else printf 'MISS  %s\n' "$marker"; failed=1; fi
done
if ! grep -Eq '\[NSEL\].*candidates=0.*selected=none' "$LOG"; then
 echo 'FAIL  QEMU unexpectedly reported a physical native GPU; C1 qualification expects no fake Radeon/Intel/NVIDIA adapter' >&2; failed=1
fi
if ! grep -Eq '\[NDOM\].*engine_qualified=true.*persistent_device_domain=false.*bus_master=false' "$LOG"; then
 echo 'FAIL  native GPU DMA safety boundary was not preserved' >&2; failed=1
fi
for bad in '[PANIC]' '[FAIL] K14.C1 native GPU binding foundation failed:'; do
 if grep -Fq "$bad" "$LOG"; then printf 'FAIL  detected: %s\n' "$bad"; failed=1; fi
done
if (( failed )); then echo 'Titanweave K14.C1 native binding/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C1 native binding/runtime qualification PASSED.'
