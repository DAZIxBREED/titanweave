#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14-serial.log}"
if [[ ! -f "$LOG" ]]; then echo "K14.A serial log not found: $LOG" >&2; exit 1; fi
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[TEST] Breakpoint exception returned successfully'
 '[COMP] surface/damage self-test:'
 '[INPT] focus/capture self-test:'
 '[FGFX] ForgeGraphics ABI v1 backend contract passed'
 '[WPS ] Workplace Shell reference preview rendered:'
 '[BUS ] ForgeBus retained'
 '[TEST] K11.1-K11.8 backend self-tests passed='
 '[GPU ] K13 topology:'
 '[GPUT] K13.B VirtIO-GPU transport ready:'
 '[GPRE] K13.C buffered presentation ready:'
 '[GRDY] K13.D resilience/multi-GPU ready:'
 '[NDRV] native backend contract self-test:'
 '[NGPU] native adapter probe:'
 '[IOMQ] native DMA admission:'
 '[NFAL] native activation deferred; K13 VirtIO-GPU and K12 GOP fallback remain armed'
 '[NATF] K14.A native GPU prerequisite foundation ready:'
 '[USER] Starting disk-backed K14 native services'
 '[USER] [displayd] K14.A native GPU activation deferred; qualified VirtIO/GOP fallback retained'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.A alive:'
 '[QUAL] K14.A native-GPU foundation runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then printf 'PASS  %s\n' "$marker"; else printf 'MISS  %s\n' "$marker"; failed=1; fi
done
# QEMU qualification must stay on the safe/deferred native path.
if ! grep -Eq '\[IOMQ\].*hardware_translation=false.*bus_master_authorized=false' "$LOG"; then
 echo 'FAIL  K14.A did not keep native DMA/bus mastering fail-closed' >&2; failed=1
fi
for bad in '[PANIC]' '[FAIL] K14.A native GPU prerequisite foundation failed:'; do
 if grep -Fq "$bad" "$LOG"; then printf 'FAIL  detected: %s\n' "$bad"; failed=1; fi
done
if (( failed )); then echo 'Titanweave K14.A native-GPU foundation/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.A native-GPU foundation/runtime qualification PASSED.'
