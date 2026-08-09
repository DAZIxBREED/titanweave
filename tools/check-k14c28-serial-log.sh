#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c28-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C28 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C27OK] K14.C27 complete Radeon driver core:'
 '[C28ME] Radeon memory manager:'
 '[C28FW] Radeon firmware manager:'
 '[C28RC] Radeon recovery manager:'
 '[C28PG] memory/firmware/recovery authority:'
 '[C28RD] K14.C28 memory+firmware+recovery ready:'
 '[C28OK] K14.C28 Radeon memory+firmware+recovery:'
 '[USER] [displayd] K14.C28 Radeon memory, firmware staging, and recovery subsystem online'
 '[USER] [displayd] K14.C28 no physical Radeon in QEMU; real GTT allocation/mapping/reclaim plus firmware parser/CRC and watchdog recovery logic qualified while hardware firmware staging is deferred'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio'
 '[KERN] K14.C28 alive:'
 '[QUAL] K14.C28 memory-firmware-recovery runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do
 if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi
done
if grep -Fq '[FAIL] K14.C28 Radeon memory+firmware+recovery failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C28 memory-firmware-recovery runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C28 memory-firmware-recovery runtime qualification PASSED.'
