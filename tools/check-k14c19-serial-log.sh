#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c19-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C19 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C18OK] K14.C18 AMD discovery snapshot-verification gate:'
 '[C19PG] physical discovery-read policy:'
 '[C19AP] VRAM aperture contract:'
 '[C19HW] physical AMD discovery snapshot:'
 '[C19RD] K14.C19 physical snapshot gate ready:'
 '[C19OK] K14.C19 physical AMD discovery snapshot gate:'
 '[USER] [displayd] K14.C19 physical AMD discovery snapshot gate online'
 '[USER] [displayd] K14.C19 no physical Radeon in QEMU; direct BAR0 discovery snapshot read remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C19 alive:'
 '[QUAL] K14.C19 physical-snapshot runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C19 physical AMD discovery snapshot gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C19 physical-snapshot runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C19 physical-snapshot runtime qualification PASSED.'
