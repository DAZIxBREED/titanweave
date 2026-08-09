#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/tools/validate-source.sh"
cat <<'MSG'
Static/source gates passed.
For the active K14.A native-GPU prerequisite runtime gate, run:
  ./tools/run-k14-qemu-native-gpu.sh
Then, if needed:
  ./tools/check-k14-serial-log.sh build/k14-serial.log
K13.D remains the frozen rollback baseline; do not overwrite it.
MSG
