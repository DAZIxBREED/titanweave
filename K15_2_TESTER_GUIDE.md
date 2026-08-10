# K15.2 Tester Guide — ForgeAudio Kernel ABI

## Run

```bash
cd ~/Downloads/titanweave-kernel-k15-2-integrated
chmod +x tools/run-k15-2-qemu-forgeaudio-abi.sh
./tools/validate-source.sh
K15_DISPLAY=none ./tools/run-k15-2-qemu-forgeaudio-abi.sh
```

The runner stops QEMU after Titanweave reaches its intentional post-userspace HALT.

## Required K15.2 evidence

The checker requires:

- the frozen K15.1 RT gate still passes;
- ForgeAudio ABI v1 comes online;
- QEMU reports `devices=0` and `fake_devices=false` rather than inventing hardware;
- illegal stream start is rejected and stream recovery passes;
- real bounded buffer write/readback passes;
- monotonic clock snapshot passes;
- bounded event queue passes;
- monotonic fence passes;
- `[K15OK] K15.2 ...` is emitted;
- `[K15ARD] ... version=1 ... fake_devices=false` is emitted;
- K14.C32 still reaches its intentional HALT;
- no `[FAIL]` line exists.

The host should finish with:

```text
Titanweave K15.2 ForgeAudio kernel ABI runtime qualification PASSED.
```

Do not start K15.3 if this gate fails.
