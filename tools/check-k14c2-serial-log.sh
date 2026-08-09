#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c2-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C2 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[IOMR] hardware translation qualification: backend=IntelVtd translated=true'
 '[NCF ] K14.C1 native binding foundation ready:'
 '[PDOM] K14.C2 persistent translated-domain surrogate:'
 '[PDRV] K14.C2 persistent-domain surrogate revoked:'
 '[AFWP] K14.C2 AMD firmware bring-up plan:'
 '[ARING] K14.C2 AMD ring plan:'
 '[ASIC] K14.C2 AMD candidate identity:'
 '[C2RD] K14.C2 native bring-up contract ready:'
 '[C2NF] K14.C2 native persistent-domain/AMD bring-up ready:'
 '[USER] [displayd] K14.C2 persistent-domain and AMD bring-up contract online'
 '[USER] [displayd] K14.C2 persistent translated-domain surrogate verified; physical Radeon domain remains fenced'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C2 alive:'
 '[QUAL] K14.C2 native bring-up runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for m in "${required[@]}"; do if grep -Fq "$m" "$LOG"; then printf 'PASS  %s\n' "$m"; else printf 'MISS  %s\n' "$m"; failed=1; fi; done
if ! grep -Eq '\[PDOM\].*epochs=3.*mappings_retained=2.*bus_master=true' "$LOG"; then echo 'FAIL  persistent-domain surrogate did not retain mappings for three DMA epochs' >&2; failed=1; fi
if ! grep -Eq '\[PDRV\].*bus_master=false.*translation_enabled=false' "$LOG"; then echo 'FAIL  persistent-domain surrogate was not revoked cleanly' >&2; failed=1; fi
if ! grep -Eq '\[C2RD\].*surrogate_domain=true.*actual_gpu_domain=false.*bus_master=false.*fallback=true' "$LOG"; then echo 'FAIL  native Radeon safety boundary was not preserved' >&2; failed=1; fi
if grep -Fq '[FAIL] K14.C2 native bring-up contract failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C2 native-domain/runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C2 native-domain/runtime qualification PASSED.'
