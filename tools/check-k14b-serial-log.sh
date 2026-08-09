#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14b-serial.log}"
if [[ ! -f "$LOG" ]]; then echo "K14.B serial log not found: $LOG" >&2; exit 1; fi
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[TEST] Breakpoint exception returned successfully'
 '[BUS ] ForgeBus retained'
 '[TEST] K11.1-K11.8 backend self-tests passed='
 '[GPUF] K13 acceleration foundation ready:'
 '[GPUT] K13.B VirtIO-GPU transport ready:'
 '[GPRE] K13.C buffered presentation ready:'
 '[GRDY] K13.D resilience/multi-GPU ready:'
 '[IOM2] K14.B translated-DMA foundation:'
 '[IOMH] Intel VT-d hardware engine:'
 '[IOVA] translated DMA map:'
 '[DMAT] EDU translated DMA round-trip verified:'
 '[IOPF] unmapped DMA denied:'
 '[INVL] VT-d context/IOTLB invalidation verified:'
 '[REVK] translated DMA test domain revoked:'
 '[IOMR] hardware translation qualification: backend=IntelVtd translated=true'
 '[IOMF] K14.B translated DMA qualification ready:'
 '[IOMQ] native DMA admission:'
 '[NATF] K14.A native GPU prerequisite foundation ready:'
 '[USER] Starting disk-backed K14 native services'
 '[USER] [displayd] K14.B display/compositor service online; translated-DMA qualification enabled'
 '[USER] [displayd] K14.B hardware-translated DMA engine qualified'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.B alive:'
 '[QUAL] K14.B translated-DMA runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then printf 'PASS  %s\n' "$marker"; else printf 'MISS  %s\n' "$marker"; failed=1; fi
done
if ! grep -Eq '\[IOMQ\].*hardware_translation=true.*device_domain_bound=false.*bus_master_authorized=false' "$LOG"; then
 echo 'FAIL  native GPU bus mastering was not kept fenced after translation-engine qualification' >&2; failed=1
fi
if ! grep -Eq '\[IOPF\].*destination_unchanged=true' "$LOG"; then
 echo 'FAIL  revoked IOVA was not proven inaccessible' >&2; failed=1
fi
for bad in '[PANIC]' '[FAIL] K14.B hardware translation qualification failed:'; do
 if grep -Fq "$bad" "$LOG"; then printf 'FAIL  detected: %s\n' "$bad"; failed=1; fi
done
if (( failed )); then echo 'Titanweave K14.B translated-DMA runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.B translated-DMA runtime qualification PASSED.'
