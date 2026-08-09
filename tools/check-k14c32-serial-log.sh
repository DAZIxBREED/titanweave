#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c32-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C32 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C31OK] K14.C31 graphics+compute execution:'
 '[C32QS] queue stability:'
 '[C32MP] memory pressure:'
 '[C32RC] recovery+interrupt stress:'
 '[C32CX] concurrency:'
 '[C32MD] display stability:'
 '[C32PM] power policy:'
 '[C32TL] telemetry/diagnostics:'
 '[C32AB] frozen GPU ABI/capabilities:'
 '[C32PG] final authority:'
 '[C32RD] K14.C32 production/stability final ready:'
 '[C32OK] K14.C32 production/stability + final K14:'
 '[USER] [displayd] K14.C32 production/stability and final Radeon foundation online'
 '[USER] [displayd] K14.C32 QEMU production gate verified: queue/memory stress, recovery+IRQ, graphics+compute/display+compute coexistence, scanout, telemetry, power, frozen GPU ABI, shader precache, multi-GPU inventory; physical Radeon stress separate'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio'
 '[KERN] K14.C32 alive:'
 '[QUAL] K14.C32 production-stability-final runtime reached intentional post-userspace halt'
 '[K14DONE] Titanweave native Radeon driver foundation operational'
 '[K15NEXT] K15 ForgeAudio is the next locked Titanweave milestone'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi; done
if grep -Fq '[FAIL] K14.C32 production/stability final failed:' "$LOG"; then failed=1; fi
# QEMU must not masquerade as physical Radeon stress evidence.
if ! grep -Fq '[C32PG] final authority: reference_production_stability=true physical_stress_qualified=false' "$LOG"; then echo 'FAIL  C32 QEMU physical-stress authority was not explicitly false.' >&2; failed=1; fi
if ((failed)); then echo 'Titanweave K14.C32 production-stability-final runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C32 production-stability-final runtime qualification PASSED.'
