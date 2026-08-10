# K15.5 — ForgeAudio PCM Format Engine

K15.5 is gate 5 of the locked 16-gate ForgeAudio stone contract. It consumes frozen K15.4 and turns the single known-good HDA qualification format into a real backend-neutral PCM format layer without implementing later mixer, resampler, graph, or userspace-server work.

## Implemented

### Canonical sample/container formats

K15.5 uses the four sample types already frozen in ForgeAudio ABI v1:

- signed 16-bit (`S16`) in a 16-bit container;
- signed 24-bit valid data in a 32-bit container (`S24In32`);
- signed 32-bit (`S32`);
- IEEE-style 32-bit floating sample storage (`F32`) as a canonical ForgeAudio memory format.

The PCM engine does not mutate the frozen ABI structs or bump ABI v1.

### Bounded interleaved / planar conversion

`forgeaudio_pcm::convert_storage_layout` performs byte-exact interleaved ↔ planar transforms for every canonical sample type. It allocates nothing, performs no filesystem/blocking work, is bounded to 2048 frames and 16 channels per call, validates every byte geometry, and preserves sample payload bytes exactly.

### Channel mapping

The engine defines explicit named channel positions and canonical maps through 16 channels. `remap_channels` maps by channel identity, not by numeric index, and zero-fills destination channels that do not exist in the source. It deliberately does not mix, downmix, or synthesize gain-weighted channels; that DSP behavior belongs to later graph/mixer gates.

### Supported-rate negotiation

The engine carries the canonical HDA PCM rate table from 8 kHz through 384 kHz, parses HDA supported-rate/valid-width bitfields, and supports:

- exact-rate negotiation, which fails closed when unsupported;
- deterministic nearest-rate negotiation, preferring the lower rate on an exact tie;
- explicit sample-format, storage-layout, and channel-count capability checks.

### HDA stream-format encoding / decoding

Integer PCM negotiations can be encoded into the 16-bit HDA stream format: base family, multiplier, divisor, sample width and `channels-1`. K15.5 round-trips all twelve canonical HDA rates and rejects reserved/non-PCM encodings. F32 remains a canonical ForgeAudio memory format but is not falsely encoded as integer HDA PCM.

### DMA geometry

The negotiated format produces exact frame stride, period byte length, period count, ring frame count and ring byte count. Geometry is bounded by the frozen K15.3 maximum of 32 periods and 1 MiB per DMA ring.

### Real HDA endpoint binding

After K15.4 registers the real QEMU HDA device and its playback/capture endpoints, K15.5 resolves those actual registry objects and negotiates the frozen proven 48 kHz / S16 / stereo format against them. It verifies HDA stream format `0x0011` and confirms the resulting frame stride agrees with ForgeAudio ABI v1 `AudioStreamConfig`.

No device or endpoint is fabricated for K15.5.

## Runtime qualification markers

A successful boot emits:

```text
[K15PCM] canonical engine:
[K15PCM] HDA format engine:
[K15PCM] layout+channel engine:
[K15PCM] DMA geometry:
[K15PCM] real HDA endpoint binding:
[K15OK] K15.5 ForgeAudio PCM format engine qualified:
[K15PR] ForgeAudio PCM ready:
```

K15.6 ForgeAudioD remains locked until Fedora/QEMU runtime qualification passes.
