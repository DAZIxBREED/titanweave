# K15.1 Tester Guide — ForgeAudio RT Foundation

## Run

```bash
cd ~/Downloads/titanweave-kernel-k15-1-integrated
chmod +x tools/run-k15-1-qemu-forgeaudio-rt.sh
./tools/validate-source.sh
K15_DISPLAY=none ./tools/run-k15-1-qemu-forgeaudio-rt.sh
```

The runner terminates QEMU automatically after Titanweave reaches its intentional post-userspace HALT.

## Pass criteria

The checker requires all of the following:

- K14 boot baseline entered.
- K15.1 RT test started at 1 kHz.
- priority-inheritance owner boost observed.
- priority-inheritance waiter ownership transfer observed.
- exactly eight periodic audio jobs completed.
- zero deadline misses.
- zero budget exhaustions.
- at least one bounded preemption deferral.
- zero preemption-guard overruns.
- `[K15OK]` and `[K15RD]` are present.
- inherited K14.C32 runtime qualification still reaches intentional HALT.
- no `[FAIL]` line exists.

The host should end with:

```text
Titanweave K15.1 ForgeAudio RT runtime qualification PASSED.
```

Do not start K15.2 if this gate fails.
