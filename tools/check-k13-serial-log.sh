#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k13-serial.log}"

if [[ ! -f "$LOG" ]]; then
  echo "Serial log not found: $LOG" >&2
  exit 2
fi

required=(
  '[BOOT] WeaveCore K13 entered from WEAVECORE.ELF'
  '[TEST] Breakpoint exception returned successfully'
  '[COMP] surface/damage self-test:'
  '[INPT] focus/capture self-test:'
  '[FGFX] ForgeGraphics ABI v1 backend contract passed'
  '[WPS ] Workplace Shell reference preview rendered:'
  '[GFX ] K13 GOP scanout online:'
  '[BUS ] ForgeBus retained'
  '[TEST] K11.1-K11.8 backend self-tests passed='
  '[STOR] K11 auto-mount retained:'
  '[GPU ] K13 topology:'
  '[VRAM] domain lifecycle self-test:'
  '[CMDQ] bounded submission self-test:'
  '[FENC] timeline self-test:'
  '[MODE] atomic modeset contract:'
  '[MGPU] transfer policy self-test:'
  '[VGPU] VirtIO-GPU candidate at'
  '[GACC] ForgeGraphics acceleration ABI v1 foundation passed transport_ready=false'
  '[GPUF] K13 acceleration foundation ready:'
  '[USER] Starting disk-backed K13 native services'
  '[USER] [displayd] K13 display/compositor service online'
  '[RECV] kernel initialization reached stable userspace handoff'
  '[KERN] K13 alive:'
  '[QUAL] K13 foundation runtime reached intentional post-userspace halt'
  '[HALT] BSP halted intentionally'
)

failed=0
for marker in "${required[@]}"; do
  if grep -Fq "$marker" "$LOG"; then
    printf 'PASS  %s\n' "$marker"
  else
    printf 'MISS  %s\n' "$marker" >&2
    failed=1
  fi
done

if grep -Fq '[PANIC]' "$LOG"; then
  echo 'FAIL  panic detected' >&2
  failed=1
fi

if (( failed != 0 )); then
  echo 'Titanweave K13.A GPU-foundation runtime qualification FAILED.' >&2
  exit 1
fi

echo 'Titanweave K13.A GPU-foundation runtime qualification PASSED.'
