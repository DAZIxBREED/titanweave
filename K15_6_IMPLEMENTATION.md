# K15.6 — ForgeAudioD

K15.6 is gate 6 of the locked 16-gate ForgeAudio stone contract. It consumes frozen K15.5 and moves ForgeAudio control ownership into a real persistent userspace server.

## Purpose

K15.1-K15.5 establish real-time scheduling, the kernel ABI, DMA transport, real HDA hardware, and the canonical PCM format engine. K15.6 adds the long-lived process that owns those resources on behalf of future applications.

This gate is intentionally a **control-plane server**, not the later application transport or DSP graph engine. K15.7 adds lock-free client/server transport. K15.8 adds executable graph nodes. K15.11 adds resampling.

## Boot service

`userspace/forgeaudiod/forgeaudiod.S` builds as `AUDIOD.ELF` and is registered as `ServiceRole::Audio` before the scripted shell. It receives only the normal console capability initially; all audio capabilities are obtained through the frozen ForgeAudio ABI.

The service is persistent. After initialization it yields cooperatively and continues to exist until K6 ends the qualification runtime. It does not exit as a success shortcut.

## Singleton kernel authority

K15.6 adds syscall 47, `SYS_AUDIO_SERVER_CONTROL`, with three bounded operations:

1. `REGISTER` — only `ServiceRole::Audio` may register; a different second PID is rejected.
2. `PUBLISH` — ForgeAudioD supplies its real device handle and bounded control counters. The kernel resolves the handle and independently validates live ForgeAudio object ownership by PID.
3. `HEARTBEAT` — accepted only after registration+publish, from the same PID, with a strictly increasing nonzero sequence.

The server cannot qualify by printing a success line. `PUBLISH` is accepted only when the process actually owns:

- at least two streams on the selected real device;
- at least one playback and one capture stream;
- at least two streams in Prepared/Running/Draining state;
- at least two buffers;
- at least one clock;
- at least one event queue;
- at least one fence;
- exactly two routes in the K15.6 control model;
- a nonzero graph-control generation;
- a nonzero recovery count.

## Real HDA ownership

On an HDA-capable K15.4+ boot, ForgeAudioD enumerates device 0, requires backend kind HDA, resolves the two registered endpoints by direction, opens the device, and creates playback/capture stream objects bound to those endpoint object IDs.

Both production-owned streams are configured to the frozen K15.5 hardware baseline:

- 48,000 Hz;
- S16;
- stereo;
- 256 frames per period;
- 1,024 frames per buffer.

They are advanced through Configured to Prepared state. K15.6 does not pretend the server has K15.7 application transport or K15.8 graph execution, so it does not manufacture client samples or claim server-driven hardware playback.

## Buffers, clock, events and fence

ForgeAudioD owns two 4,096-byte ABI buffers with a four-byte stereo-S16 frame stride, one 48 kHz clock object, one event queue and one fence. These are real kernel objects associated with the ForgeAudioD PID and are closed through normal process teardown.

## Bounded routing/control metadata

The userspace server materializes two fixed-size route records:

- playback endpoint -> playback stream -> playback server buffer;
- capture endpoint -> capture stream -> capture server buffer.

The table is bounded to exactly two records in K15.6 and starts at graph generation 1. It is control metadata only: there is no mixer, gain node, resampler, application ring or DSP graph hidden in this milestone.

## Recovery proof

ForgeAudioD creates a temporary playback stream and intentionally attempts `StartStream` before configure/prepare. The kernel must return `ERROR_INVALID_STATE`. The daemon then closes the invalid control object, opens a replacement stream, configures and prepares it correctly, and releases it cleanly while retaining its two production-owned prepared streams.

This is real control-plane recovery/rebuild behavior rather than a fabricated `recovery=true` flag. The server publishes recovery count 1 only after those syscalls succeed.

## Persistence proof

After the kernel accepts the ownership publication, ForgeAudioD calls `SYS_YIELD`. Only when the scheduler later returns to that same process does the daemon submit heartbeat sequence 1. K6 requires both `forgeaudiod_ready` and `forgeaudiod_heartbeat` whenever a real ForgeAudio device exists.

Thus a server that initializes and then dies, blocks permanently, or never receives another scheduling slice cannot qualify.

## Backward qualification behavior

K15.1-K15.3 QEMU runners intentionally contain no HDA device. In that case ForgeAudioD reports a dormant no-hardware state and remains persistent, while K6 does not require the K15.6 ownership proof. This preserves those frozen no-hardware qualification semantics.

When a real ForgeAudio device exists (K15.4+ QEMU hardware model or future physical backend), K6 requires the full K15.6 ready+heartbeat proof.

## Runtime qualification

A successful K15.6 boot must include kernel-backed and userspace evidence such as:

```text
[K15D] ForgeAudioD registered: ... singleton=true userspace=true
[USER] [forgeaudiod] K15.6 device ownership: HDA=true playback=true capture=true singleton=true
[USER] [forgeaudiod] K15.6 objects: streams=2 buffers=2 clocks=1 events=1 fences=1 prepared=true
[USER] [forgeaudiod] K15.6 control plane: routes=2 graph_generation=1 bounded=true no_mixing=true no_resampling=true
[USER] [forgeaudiod] K15.6 recovery: invalid_start_rejected=true stream_rebuilt=true recoveries=1
[K15D] ForgeAudioD ownership verified: ...
[USER] [forgeaudiod] K15.6 ForgeAudioD ready: ...
[K15D] ForgeAudioD heartbeat: ... persistent=true
[USER] [forgeaudiod] K15.6 persistent heartbeat: sequence=1 scheduler_return=true
[K15OK] K15.6 ForgeAudioD userspace audio server qualified: ...
[K15SR] ForgeAudioD ready: ...
```

K15.7 remains locked until this runtime gate passes.
