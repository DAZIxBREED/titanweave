# K15.3 Tester Guide — ForgeAudio Audio DMA Transport

## Run

```bash
cd ~/Downloads/titanweave-kernel-k15-3-integrated
chmod +x tools/run-k15-3-qemu-forgeaudio-dma.sh
./tools/validate-source.sh
K15_DISPLAY=none ./tools/run-k15-3-qemu-forgeaudio-dma.sh
```

The runner stops QEMU after Titanweave reaches its intentional post-userspace HALT.

## Required K15.3 evidence

The checker requires:

- K15.1 RT remains qualified;
- K15.2 ABI remains qualified;
- K14.B again proves actual translated DMA with QEMU EDU/VT-d;
- a real physically contiguous kernel-DMA ring is allocated/mapped;
- untranslated hardware arming is rejected;
- the QEMU path reports `audio_hw_deferred=true` / `hardware_audio=false` instead of inventing an HDA device;
- cyclic period accounting crosses at least two wraps;
- cumulative frame position advances;
- ownership enforcement is enabled;
- exactly one playback underrun is detected;
- exactly one capture overrun is detected;
- `[K15OK] K15.3 ...` and `[K15DR] ...` are emitted;
- K14.C32 still reaches the intentional halt;
- no `[FAIL]` line exists.

The host should finish with:

```text
Titanweave K15.3 ForgeAudio audio DMA transport runtime qualification PASSED.
```

Do not start K15.4 if this gate fails.
