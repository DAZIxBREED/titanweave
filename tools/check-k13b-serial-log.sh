#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k13b-serial.log}"

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
  '[VPCI] modern capabilities + VERSION_1 negotiated:'
  '[VQ  ] controlq='
  '[VDMA] ForgeBus bounded DMA ownership online:'
  '[SCAN] VirtIO-GPU resource 1 scanout='
  '[GACC] ForgeGraphics acceleration ABI v1 transport passed transport_ready=true backend=virtio-gpu-modern'
  '[GPUT] K13.B VirtIO-GPU transport ready:'
  '[USER] Starting disk-backed K13 native services'
  '[USER] [displayd] K13.B display/compositor service online'
  '[USER] [displayd] K13.B VirtIO-GPU command transport online'
  '[RECV] kernel initialization reached stable userspace handoff'
  '[KERN] K13.B alive:'
  '[QUAL] K13.B transport runtime reached intentional post-userspace halt'
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

for bad in '[PANIC]' '[FAIL] K13.B VirtIO-GPU transport failed:'; do
  if grep -Fq "$bad" "$LOG"; then
    echo "FAIL  detected: $bad" >&2
    failed=1
  fi
done

if (( failed != 0 )); then
  echo 'Titanweave K13.B transport/runtime qualification FAILED.' >&2
  exit 1
fi

echo 'Titanweave K13.B transport/runtime qualification PASSED.'
