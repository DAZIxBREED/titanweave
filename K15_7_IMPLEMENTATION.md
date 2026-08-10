# K15.7 — ForgeAudio Lock-Free Audio Transport

K15.7 is gate 7 of the locked 16-gate ForgeAudio stone contract. It consumes frozen K15.6 ForgeAudioD and implements the bounded application/server transport layer required before the K15.8 graph engine.

## Scope

K15.7 implements **bounded application/server audio rings** and bounded command queues with sequence/generation tracking and dead-client isolation. It does not implement DSP graph nodes, gain, mixing, resampling, sample-accurate graph switching, or later latency/hotplug policy.

## Lock-free SPSC transport core

`kernel/weavecore/src/forgeaudio_transport.rs` provides four fixed transport sessions. Each session contains:

- one application -> ForgeAudioD playback PCM ring;
- one ForgeAudioD -> application capture PCM ring;
- one application -> ForgeAudioD command queue;
- one ForgeAudioD -> application command/acknowledgement queue.

PCM rings contain exactly four 1,024-byte blocks. Command queues contain exactly sixteen 32-byte records. Producer/consumer hot paths use monotonic atomic `head`/`tail` sequences with Acquire/Release ordering. They do not allocate, take a mutex/SpinLock, perform filesystem I/O, or sleep.

Each session has an atomic state, exact client PID, exact server PID, and generation. A transport syscall is accepted only for the correct role/PID/session/generation.

## Real userspace client/server proof

K15.7 adds `AUDIOCLT.ELF` as a tenth real boot service, after ForgeAudioD and before the shell. The application client uses syscall 48 to attach to the already registered and persistent ForgeAudioD process.

The client fills the entire depth-16 command queue, verifies command 17 returns `WOULD_BLOCK`, then receives sixteen exact sequence/generation acknowledgements and verifies the queue becomes empty.

It then completes three full four-slot playback/capture laps. Twelve deterministic 1,024-byte PCM blocks move application -> ForgeAudioD -> application. ForgeAudioD validates every input byte and returns the exact block without DSP. Both directions cross three ring wraps; full and empty boundaries are exercised on every lap.

## Dead-client isolation

The client exits normally after the data proof. Process teardown calls `forgeaudio_transport::detach_process(pid)`. Only the matching session is marked dead, its generation advances, its fixed ring storage is wiped/reset, and ForgeAudioD remains alive.

ForgeAudioD discovers the dead session, deliberately attempts an operation with the stale generation and requires rejection, then reaps the session only with the new generation. Global bounded qualification counters remain available after per-session wipe so the kernel can verify the completed transport evidence.

ForgeAudioD yields after dead-client cleanup, returns as the same persistent PID, advances its K15.6 heartbeat from sequence 1 to 2, and only then asks the kernel to qualify K15.7.

## Qualification contract

The kernel requires exactly one attach, twelve playback blocks, twelve capture blocks, sixteen command round trips, three playback wraps, three capture wraps, two command-ring wraps, bounded full/empty events, at least one stale-generation rejection, exactly one dead client, and exactly one generation advance.

A successful kernel emits:

```text
[K15LF] transport attached: ... lock_free=true
[K15LF] dead client isolated: ... rings_reset=true server_alive=true
[K15OK] K15.7 ForgeAudio lock-free transport qualified: ...
[K15LR] ForgeAudio lock-free transport ready: ... SPSC=true atomics=true allocation_free=true server_persistent=true
```

K15.8 — ForgeAudio Graph Engine remains locked until K15.7 passes Fedora/QEMU runtime qualification.
