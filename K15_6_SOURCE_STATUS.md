# Titanweave K15.6 ForgeAudioD — Source Status

Status: **QUALIFIED / FROZEN**

Baseline: qualified/frozen K15.5.

K15.6 implements the first real persistent ForgeAudio userspace server. It does not move DSP, lock-free application transport, mixing, resampling, or graph execution forward from their locked later gates.

Implemented in this gate:

- ninth boot service: `AUDIOD.ELF` / `forgeaudiod`;
- dedicated `ServiceRole::Audio` and persistent scheduler lifetime;
- syscall 47 ForgeAudioD singleton registration / publish / heartbeat control;
- conditional K6 qualification dependency when real ForgeAudio hardware exists;
- honest dormant service behavior when pre-HDA runners have no audio device;
- real ForgeAudio ABI v1 query from userspace;
- real HDA device enumeration and backend identity check;
- playback and capture endpoint discovery by direction;
- device object ownership through a real process audio handle;
- one prepared playback stream and one prepared capture stream at 48 kHz / S16 / stereo;
- two bounded 4096-byte server buffers with 4-byte frame stride;
- server-owned 48 kHz clock, event queue and fence;
- bounded two-route control metadata with graph generation 1;
- control-plane recovery proof: illegal unprepared start rejection, close, rebuild, configure, prepare, release;
- kernel-side live ownership cross-validation by PID/device before accepting readiness;
- explicit post-yield heartbeat proving the daemon remains persistent under K6 scheduling;
- failure propagation when required ForgeAudioD exits before qualification;
- K15.6 source/serial qualification tools.

The packaging environment can assemble/link userspace but does not provide Rust/Cargo/QEMU. Fedora must compile and runtime-qualify K15.6 before it is frozen and K15.7 begins.
