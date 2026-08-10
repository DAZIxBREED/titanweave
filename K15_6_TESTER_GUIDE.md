# K15.6 Tester Guide — ForgeAudioD

## Run

```bash
cd ~/Downloads/titanweave-kernel-k15-6-integrated
chmod +x tools/run-k15-6-qemu-forgeaudiod.sh
./tools/validate-source.sh
K15_DISPLAY=none ./tools/run-k15-6-qemu-forgeaudiod.sh
```

The runner builds the current tree, boots QEMU with the frozen K15.4 ICH9-HDA/hda-duplex model and Intel VT-d configuration, and stops QEMU after Titanweave reaches its intentional HALT marker.

## Required evidence

The checker requires:

- inherited K15.1-K15.5 remains green;
- `AUDIOD.ELF` is loaded as a real process;
- ForgeAudioD singleton registration succeeds;
- the daemon owns the real HDA device and resolves playback/capture endpoints;
- two prepared streams exist under the ForgeAudioD PID;
- two bounded buffers, one clock, one event queue and one fence exist;
- two bounded route records and graph generation 1 are published;
- illegal unprepared start returns the expected invalid-state error and a replacement stream is rebuilt/prepared/released;
- the kernel independently validates the process's live ForgeAudio ownership;
- ForgeAudioD yields and later produces a heartbeat from the same persistent process;
- K6 waits for the required ForgeAudioD heartbeat before ending userspace;
- `[K15OK] K15.6 ...` and `[K15SR] ...` are emitted;
- K14.C32 still reaches intentional HALT;
- no `[FAIL]` line exists.

The host should finish with:

```text
Titanweave K15.6 ForgeAudioD runtime qualification PASSED.
```

Do not start K15.7 if this gate fails.
