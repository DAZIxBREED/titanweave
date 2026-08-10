# Titanweave K15.5 ForgeAudio PCM Format Engine — Source Status

Status: **QUALIFIED / FROZEN**

Baseline: qualified/frozen K15.4.

Implemented in this gate:

- canonical S16, S24-in-32, S32 and F32 ForgeAudio memory formats;
- 12-rate HDA canonical sample-rate table (8 kHz through 384 kHz);
- exact HDA supported-rate and valid-width capability parsing;
- bounded exact/nearest supported-rate negotiation;
- explicit 1..16-channel named channel maps;
- bounded allocation-free interleaved ↔ planar conversion;
- named-position channel remap with absent-channel zero-fill and no hidden mixing;
- HDA PCM stream-format encode/decode and roundtrip verification;
- K15.3-compatible period/ring geometry with 32-period / 1 MiB bounds;
- binding to real K15.4 HDA playback/capture registry endpoints;
- frozen ABI-v1 frame-stride agreement check;
- rejection of unsupported exact rates, channels, formats and storage layouts;
- K15.5 source and serial qualification tools.

No K15.6 ForgeAudioD, graph, resampler or mixer implementation is included.

Fedora/QEMU must runtime-qualify K15.5 before it is frozen and K15.6 is unlocked.
