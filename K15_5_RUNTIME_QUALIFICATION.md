# Titanweave K15.5 ForgeAudio PCM Format Engine — Runtime Qualification

Status: **QUALIFIED / FROZEN**

Date: 2026-08-10

Fedora/QEMU runtime qualification PASSED.

Qualified evidence:

- inherited K15.1-K15.4 qualification retained;
- four canonical PCM sample formats (S16, S24-in-32, S32, F32);
- twelve canonical HDA sample rates;
- up to sixteen channels;
- bounded allocation-free interleaved/planar conversion;
- named channel mapping with zero-fill and no hidden mixing;
- exact and nearest supported-rate negotiation;
- twelve HDA rate encode/decode round trips;
- exact 96 kHz / S24-in-32 / 6-channel vector encoded as `0x0835`;
- 50 kHz nearest-rate negotiation to 48 kHz;
- unsupported exact rate/channel requests rejected fail-closed;
- K15.3-compatible period/ring geometry (256 frames, 6144 bytes, 4 periods, 24576-byte ring);
- binding to the real K15.4 HDA playback/capture endpoints;
- 48 kHz / S16 / stereo endpoint encoded as `0x0011`;
- no fabricated audio device;
- K14.C32 qualification retained;
- intentional Titanweave HALT reached;
- raw QEMU exit status 0.

K15.5 is QUALIFIED / FROZEN.

K15.6 — ForgeAudioD is unlocked.
