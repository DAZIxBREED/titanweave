# Titanweave K15.8 ForgeAudio Graph Engine — Tester Guide

## 1. Source validation

```bash
cd ~/Downloads/titanweave-kernel-k15-8-integrated
./tools/validate-source.sh
```

Expected final source line:

```text
Titanweave K1-K15.8 integrated source validation passed; K15.8 runtime qualification pending.
```

The K15.8 source test also verifies hashes of the user-provided K15.7-qualified kernel/ABI/transport baseline. A frozen-file mismatch is a hard failure.

## 2. QEMU runtime qualification

```bash
K15_DISPLAY=none ./tools/run-k15-8-qemu-forgeaudio-graph.sh
```

Required new K15.8 evidence includes:

```text
[K15GR] graph compiled: generation=1 nodes=8 edges=8 order=8 ...
[K15GR] node execution: Input=4 Output=4 Gain=4 Mixer=4 Splitter=4 ChannelMapper=4 FormatConverter=4 Meter=4
[K15GR] PCM verified: blocks=4 frames=1024 channels=2 format=S16 output_verified=true format_roundtrips=4 meter_peak=400 meter_sum_abs=204800
[K15GR] ForgeAudio graph proof complete: ...
[K15OK] K15.8 ForgeAudio Graph Engine qualified: ...
[K15GR] ForgeAudio Graph Engine ready: ... sample_accurate_switching=false resampling=false
[USER] [forgeaudiod] K15.8 graph engine ready: ...
```

The inherited K15.7 proof and intentional K14.C32 qualification halt must also remain green. Runtime ordering is required to be K15.7 qualification -> K15.8 graph qualification -> final QUAL -> HALT.

## 3. Freeze rule

Do not merge/freeze/tag K15.8 until the final line is:

```text
Titanweave K15.8 ForgeAudio Graph Engine runtime qualification PASSED.
```
