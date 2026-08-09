#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c18-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C18 serial log not found: $LOG" >&2; exit 1; }
markers=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C17OK] K14.C17 AMD IP-discovery/Navi48 base-resolution gate:'
 '[C18CK] discovery checksum verifier:'
 '[C18PG] snapshot acquisition policy:'
 '[C18HW] AMD discovery snapshot:'
 '[C18RD] K14.C18 snapshot-verification gate ready:'
 '[C18OK] K14.C18 AMD discovery snapshot-verification gate:'
 '[USER] [displayd] K14.C18 AMD discovery snapshot-verification gate online'
 '[USER] [displayd] K14.C18 no physical Radeon in QEMU; bounded live discovery snapshot acquisition remains safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[KERN] K14.C18 alive:'
 '[QUAL] K14.C18 snapshot-verification runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for m in "${markers[@]}"; do if grep -Fq "$m" "$LOG"; then echo "PASS  $m"; else echo "FAIL  $m" >&2; failed=1; fi; done
if grep -Fq '[FAIL] K14.C18 AMD discovery snapshot-verification gate failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C18 snapshot-verification runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C18 snapshot-verification runtime qualification PASSED.'
