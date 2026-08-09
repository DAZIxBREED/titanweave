#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c27-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C27 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C26OK] K14.C26 final reviewed GFX12 MMIO allowlist/read-only completion gate:'
 '[C27DV] Radeon driver core:'
 '[C27RS] Radeon resource ownership/topology:'
 '[C27MM] Radeon reviewed MMIO service:'
 '[C27IR] Radeon interrupt core:'
 '[C27ER] Radeon error/reset coordinator:'
 '[C27PG] driver-core authority:'
 '[C27RD] K14.C27 complete driver-core ready:'
 '[C27OK] K14.C27 complete Radeon driver core:'
 '[USER] [displayd] K14.C27 complete Radeon driver core online'
 '[USER] [displayd] K14.C27 no physical Radeon in QEMU; operational driver-core software paths qualified and physical ownership/MMIO route safely deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio'
 '[KERN] K14.C27 alive:'
 '[QUAL] K14.C27 complete-radeon-driver-core runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C27 complete Radeon driver core failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C27 complete-radeon-driver-core runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C27 complete-radeon-driver-core runtime qualification PASSED.'
