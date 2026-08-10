# K15.7 Tester Guide — ForgeAudio Lock-Free Audio Transport

## Run

```bash
cd ~/Downloads/titanweave-kernel-k15-7-integrated
./tools/validate-source.sh
K15_DISPLAY=none ./tools/run-k15-7-qemu-forgeaudio-lockfree.sh
```

## Required evidence

The runtime checker requires frozen K15.1-K15.6 evidence plus:

- `AUDIOCLT.ELF` loaded as a distinct process;
- one client/server session at generation 1;
- depth-16 command queue full/empty and sixteen round trips;
- twelve playback and twelve capture blocks;
- three playback and three capture wraps;
- deterministic returned PCM bytes;
- automatic dead-client generation advance to generation 2;
- stale generation rejection;
- dead-session reap without ForgeAudioD termination;
- ForgeAudioD heartbeat sequence 2 after isolation;
- kernel lock-free transport qualification and ready markers;
- K14.C32 qualification and intentional HALT retained;
- no `[FAIL]` marker.

K15.8 must not begin if this gate fails.
