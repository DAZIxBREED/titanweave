#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${K15_8_SERIAL_LOG:-$ROOT/build/k15-8-serial.log}"

export K15_7_SERIAL_LOG="$LOG"
export K15_7_NVME_IMAGE="${K15_8_NVME_IMAGE:-$ROOT/build/k15-8-nvme-test.img}"
export K15_7_NVME_SIZE="${K15_8_NVME_SIZE:-256M}"
export K15_7_TIMEOUT="${K15_8_TIMEOUT:-150}"

cat <<MSG
Titanweave K15.8 ForgeAudio Graph Engine QEMU qualification
  serial log       : $LOG
  display          : ${K15_DISPLAY:-none}
  inherited runner : K15.7 HDA + real ForgeAudioD + audio-client transport

K15.8 consumes frozen/qualified K15.7. After the real lock-free transport proof,
ForgeAudioD triggers the bounded graph engine. Qualification requires a compiled
cycle-free eight-node DAG with operational Input, Output, Gain, Mixer, Splitter,
Channel Mapper, Format Converter and Meter nodes. Four 1024-byte S16 stereo
blocks must execute through every node with deterministic output and meter data.
The graph process path must remain allocation-free and lock-free. K15.9 sample-
accurate graph switching and K15.11 production resampling remain intentionally
unimplemented in this gate.
MSG

# Reuse the proven K15.7 hardware/runtime harness so K15.8 cannot accidentally
# weaken the HDA, transport, dead-client or persistent-server baseline.
"$ROOT/tools/run-k15-7-qemu-forgeaudio-lockfree.sh"
"$ROOT/tools/check-k15-8-serial-log.sh" "$LOG"
