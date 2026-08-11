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
  inherited runner : frozen K15.7 HDA + ForgeAudioD + audio-client transport

K15.8 preserves the Fedora-qualified K15.1-K15.7 kernel/ABI/transport baseline
byte-for-byte. The graph engine lives in ForgeAudioD userspace. After all K15.7
command, PCM-ring, dead-client and heartbeat operations succeed—but before the
kernel closes K15.7's aggregate gate—ForgeAudioD compiles and executes an
8-node bounded DAG: Input, Output, Gain, Mixer, Splitter, Channel Mapper,
Format Converter and Meter. Four 1024-byte S16 stereo blocks must pass exact
output, format-roundtrip and meter verification. K15.9 sample-accurate graph
switching and K15.11 production resampling remain intentionally unimplemented.
MSG

"$ROOT/tools/run-k15-7-qemu-forgeaudio-lockfree.sh"
"$ROOT/tools/check-k15-8-serial-log.sh" "$LOG"
