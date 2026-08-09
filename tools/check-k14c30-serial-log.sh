#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k14c30-serial.log}"
[[ -f "$LOG" ]] || { echo "K14.C30 serial log not found: $LOG" >&2; exit 1; }
required=(
 '[BOOT] WeaveCore K14 entered from WEAVECORE.ELF'
 '[C29OK] K14.C29 Radeon rings+queues+fences+DMA:'
 '[C30ED] EDID/mode engine:'
 '[C30CN] connector topology:'
 '[C30SC] double-buffer scanout:'
 '[C30MS] atomic modeset:'
 '[C30HP] hotplug engine:'
 '[C30DC] DCN401 resource authority:'
 '[C30PG] display authority:'
 '[C30RD] K14.C30 complete basic display ready:'
 '[C30OK] K14.C30 complete basic display engine:'
 '[USER] [displayd] K14.C30 complete basic display engine online'
 '[USER] [displayd] K14.C30 QEMU/GOP backend verified: EDID parser, connector/CRTC/plane model, double-buffered GTT scanout, live framebuffer page flips, atomic mode rollback, and hotplug bookkeeping operational'
 '[RECV] kernel initialization reached stable userspace handoff'
 '[K14FOUND] K14.C26 native Radeon MMIO foundation frozen; fixed roadmap continues K14 Radeon through C32 before K15 ForgeAudio'
 '[KERN] K14.C30 alive:'
 '[QUAL] K14.C30 complete-basic-display-engine runtime reached intentional post-userspace halt'
 '[HALT] BSP halted intentionally'
)
failed=0
for marker in "${required[@]}"; do if grep -Fq "$marker" "$LOG"; then echo "PASS  $marker"; else echo "FAIL  $marker" >&2; failed=1; fi; done
if grep -Fq '[FAIL] K14.C30 complete basic display engine failed:' "$LOG"; then failed=1; fi
if ((failed)); then echo 'Titanweave K14.C30 complete-basic-display-engine runtime qualification FAILED.' >&2; exit 1; fi
echo 'Titanweave K14.C30 complete-basic-display-engine runtime qualification PASSED.'
