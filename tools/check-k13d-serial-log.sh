#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k13d-serial.log}"
if [[ ! -f "$LOG" ]]; then
  echo "K13.D serial log not found: $LOG" >&2
  exit 1
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
  '[PACE] compositor pacing contract:'
  '[FBCK] presentation watchdog policy:'
  '[VGPU] VirtIO-GPU candidate at'
  '[GACC] ForgeGraphics acceleration ABI v1 foundation passed transport_ready=false'
  '[GPUF] K13 acceleration foundation ready:'
  '[VPCI] modern capabilities + VERSION_1 negotiated:'
  '[VQ  ] controlq='
  '[VDMA] ForgeBus bounded DMA ownership online:'
  '[SCAN] VirtIO-GPU resource 1 scanout='
  '[GACC] ForgeGraphics acceleration ABI v1 transport passed transport_ready=true backend=virtio-gpu-modern'
  '[GPUT] K13.B VirtIO-GPU transport ready:'
  '[PRES] triple-buffered compositor scanout online:'
  '[DMG ] dirty-region GPU uploads verified:'
  '[PFEN] fence-verified presentation complete:'
  '[FBCK] GOP fallback remains armed after accelerated presentation: true'
  '[GCOMP] K13.C compositor presentation ready:'
  '[GPRE] K13.C buffered presentation ready:'
  '[RSLN] GPU health/rebind state machine:'
  '[HOTG] PCIe GPU hotplug policy self-test:'
  '[MOUT] multi-scanout policy self-test:'
  '[MGP2] multi-GPU presentation policy:'
  '[SOAK] presentation stress:'
  '[DLOS] controlled device-loss fence:'
  '[REBD] transport rearm verified:'
  '[GRDY] K13.D resilience/multi-GPU ready:'
  '[USER] Starting disk-backed K13 native services'
  '[USER] [displayd] K13.D display/compositor service online; resilience enabled'
  '[USER] [displayd] K13.D VirtIO-GPU command transport online; resilience enabled'
  '[UPRS] DISPLAYD present:'
  '[USER] [displayd] K13.D capability-mediated buffered present verified'
  '[URCV] DISPLAYD recovery:'
  '[USER] [displayd] K13.D capability-mediated GPU recovery verified'
  '[RECV] kernel initialization reached stable userspace handoff'
  '[KERN] K13.D alive:'
  '[QUAL] K13.D robustness runtime reached intentional post-userspace halt'
  '[HALT] BSP halted intentionally'
)

failed=0
for marker in "${required[@]}"; do
  if grep -Fq "$marker" "$LOG"; then
    printf 'PASS  %s\n' "$marker"
  else
    printf 'MISS  %s\n' "$marker"
    failed=1
  fi
done

if ! grep -Eq '\[MGP2\].*secondary=[1-9][0-9]*' "$LOG"; then
  echo 'FAIL  K13.D QEMU qualification did not observe a secondary GPU candidate' >&2
  failed=1
fi

for bad in \
  '[PANIC]' \
  '[FAIL] K13.D resilience/multi-GPU qualification failed:' \
  '[FAIL] K13.D resilience path did not become ready' \
  '[FBCK] DISPLAYD GPU recovery failed:' \
  '[displayd] GPU recovery unavailable; GOP fallback retained'; do
  if grep -Fq "$bad" "$LOG"; then
    printf 'FAIL  detected: %s\n' "$bad"
    failed=1
  fi
done

if (( failed )); then
  echo 'Titanweave K13.D robustness/runtime qualification FAILED.' >&2
  exit 1
fi

echo 'Titanweave K13.D robustness/runtime qualification PASSED.'
