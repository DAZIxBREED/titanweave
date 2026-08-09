#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${1:-$ROOT/build/k11-serial.log}"
OUT="${2:-$ROOT/build/K11_TEST_REPORT.txt}"
{
  echo 'TITAN//WEAVE K11 TEST REPORT'
  echo "generated: $(date -Iseconds 2>/dev/null || date)"
  echo
  echo '== Host =='
  uname -a || true
  echo
  echo '== CPU =='
  lscpu 2>/dev/null || true
  echo
  echo '== Toolchain =='
  rustc --version 2>&1 || true
  cargo --version 2>&1 || true
  qemu-system-x86_64 --version 2>&1 | head -1 || true
  echo
  echo '== K11 source gates =='
  "$ROOT/tools/validate-source.sh" 2>&1 || true
  echo
  echo '== Serial qualification =='
  "$ROOT/tools/check-serial-log.sh" "$LOG" 2>&1 || true
  echo
  echo '== Serial log =='
  cat "$LOG" 2>/dev/null || echo "No serial log at $LOG"
} > "$OUT"
echo "$OUT"
