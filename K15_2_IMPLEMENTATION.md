# K15.2 — ForgeAudio Kernel ABI

K15.2 is gate 2 of the locked 16-gate ForgeAudio stone contract. It builds on the qualified/frozen K15.1 RT foundation and defines the permanent v1 kernel/userspace audio object contract without pretending that QEMU contains physical audio hardware.

## Implemented

### Shared ABI crate

`libraries/forgeaudio-abi` is the single source of truth shared by WeaveCore and userspace. It freezes `FORGEAUDIO_ABI_VERSION = 1`, feature bits, object kinds, stream states, directions, sample-format identifiers, control operations, fixed C-layout records, and compile-time structure-size assertions.

The v1 object model is:

- `AudioDevice`
- `AudioEndpoint`
- `AudioStream`
- `AudioBuffer`
- `AudioClock`
- `AudioEvent`
- `AudioFence`

### Kernel object manager

`kernel/weavecore/src/forgeaudio.rs` implements bounded static object tables with explicit capacities and failure results. It contains real lifecycle logic for device registration/open references, endpoint registration, stream state transitions, in-kernel audio buffers, monotonic clocks, bounded event FIFOs and monotonic fences.

No audio device is fabricated during K15.2. The runtime registry remains empty until a real backend registers hardware. K15.4 will use the registration API for HDA.

### Real bounded buffer objects

K15.2 audio buffers own real kernel memory from a fixed bounded pool. Reads/writes perform bounds, access and committed-data checks. Close wipes the used bytes before releasing the slot. K15.3 may attach DMA transport to these object semantics without changing the v1 ABI.

### Stream lifecycle

The stream lifecycle rejects illegal transitions and implements:

`Created -> Configured -> Prepared -> Running -> Draining/Stopped`

with explicit fault and recovery behavior. Hardware execution is not claimed at this gate; no stream can be opened without a registered real device and endpoint.

### Clock, event and fence objects

- Clocks derive timestamps and frame positions from the qualified K15.1 1 kHz monotonic RT clock.
- Event objects are bounded FIFO queues with monotonic sequence numbers.
- Fence objects enforce monotonic completion and only signal when the target value is reached.

### Syscall ABI

The permanent v1 syscall numbers are:

- `44` — `SYS_AUDIO_ABI_QUERY`
- `45` — `SYS_AUDIO_ENUMERATE`
- `46` — `SYS_AUDIO_CONTROL`

The control ABI supports device open, stream open/configure/prepare/start/stop/drain/recover, position query, buffer/clock/event/fence creation, event polling, fence query and object close. User handles carry rights and are automatically released when a process exits.

`AudioControlRequest` configure semantics are frozen for v1:

- `handle`: stream handle
- `flags`: stream flags
- `object_id`: `AudioDirection` value
- `arg0`: `AudioSampleFormat` value
- `arg1`: sample rate in Hz
- `arg2`: channel count
- `arg3[31:0]`: period frames
- `arg3[63:32]`: buffer frames

### Runtime qualification

The boot-time K15.2 self-test requires:

- ABI version/features match the shared v1 contract;
- hardware registry contains zero invented devices in QEMU;
- nonexistent hardware open is rejected;
- illegal stream start is rejected and the complete state/recovery machine works;
- a real bounded buffer writes and reads back a deterministic byte pattern;
- a monotonic 48 kHz audio clock snapshots correctly;
- an event is queued/polled exactly once;
- a fence remains unsignaled below target and signals exactly at target;
- temporary K15.2 objects are fully released before normal boot continues.

A passing kernel emits:

```text
[K15OK] K15.2 ForgeAudio kernel ABI qualified: ABI=v1 device+endpoint+stream+buffer+clock+event+fence lifecycle real bounded=true fake_devices=false
[K15ARD] ForgeAudio ABI ready: version=1 ... real_devices=0 fake_devices=false
```

The inherited K15.1 and K14.C32 runtime qualification markers must also remain clean.
