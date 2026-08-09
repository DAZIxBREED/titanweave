#!/usr/bin/env bash
set -euo pipefail
LOG="${1:-build/k11-serial.log}"

if [[ ! -f "$LOG" ]]; then
  echo "Serial log not found: $LOG" >&2
  exit 2
fi

required=(
  '[BOOT] WeaveCore K11 entered from WEAVECORE.ELF'
  '[TEST] Breakpoint exception returned successfully'
  '[BUS ] ForgeBus retained'
  '[TEST] K11.1-K11.8 backend self-tests passed='
  '[STOR] K11 auto-mount retained:'
  '[USER] Starting disk-backed K11 native services'
  '[RECV] kernel initialization reached stable userspace handoff'
  '[KERN] K11 alive:'
  '[QUAL] K11 runtime reached intentional post-userspace halt'
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
if grep -Fq '[FAIL]' "$LOG"; then
  echo 'WARN  one or more [FAIL] markers are present; inspect the log.' >&2
fi

if (( failed != 0 )); then
  echo "Titanweave K11 serial qualification FAILED." >&2
  exit 1
fi

echo "Titanweave K11 serial milestone qualification PASSED."
