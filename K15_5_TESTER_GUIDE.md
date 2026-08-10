# K15.5 Tester Guide — ForgeAudio PCM Format Engine

## Run

```bash
cd ~/Downloads/titanweave-kernel-k15-5-integrated
chmod +x tools/run-k15-5-qemu-forgeaudio-pcm.sh
./tools/validate-source.sh
K15_DISPLAY=none ./tools/run-k15-5-qemu-forgeaudio-pcm.sh
```

The runner reuses the proven K15.4 QEMU HDA topology because K15.5 must bind to the real HDA endpoints registered by frozen K15.4.

## Required evidence

The checker requires:

- K15.1, K15.2, K15.3 and K15.4 remain qualified;
- four canonical ForgeAudio sample/container formats are active;
- all 12 canonical HDA rates encode/decode exactly;
- all four sample types survive interleaved → planar → interleaved byte-exactly;
- named channel mapping works and zero-fills absent channels without mixing;
- exact negotiation succeeds for a supported 96 kHz/S24-in-32/6-channel vector;
- nearest-rate negotiation maps 50 kHz to 48 kHz deterministically;
- unsupported exact rate/channel vectors are rejected;
- period/ring geometry is exact and inside frozen K15.3 bounds;
- the engine resolves the actual K15.4 HDA playback and capture endpoints;
- the real HDA endpoint negotiates 48 kHz/S16/stereo to stream format `0x0011`;
- `fake_device=false`;
- K14.C32 still reaches intentional HALT;
- no `[FAIL]` line exists.

The host must finish with:

```text
Titanweave K15.5 ForgeAudio PCM format engine runtime qualification PASSED.
```

Do not start K15.6 if this gate fails.
