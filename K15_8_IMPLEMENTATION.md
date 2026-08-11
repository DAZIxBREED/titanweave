# K15.8 — ForgeAudio Graph Engine

K15.8 is gate 8 of the locked 16-gate ForgeAudio stone contract. It consumes the user-confirmed Fedora/QEMU-qualified K15.7 lock-free transport baseline and adds the first operational ForgeAudio processing graph.

## Frozen-baseline rule

The qualified K15.1-K15.7 kernel, ABI, transport, audio-client and syscall definitions are **byte-for-byte unchanged** in this candidate. K15.8 is implemented inside the existing ForgeAudioD userspace server, which already owns graph control and routing under K15.6.

This avoids perturbing the qualified K15.1 real-time scheduler/code layout while placing graph execution in the correct userspace owner.

## Operational nodes

K15.8 implements exactly the stone-contract nodes:

- Input
- Output
- Gain
- Mixer
- Splitter
- Channel Mapper
- Format Converter
- Meter

The bounded qualification DAG is:

```text
Input -> Gain -> Splitter -> Channel Mapper --\
                         \-> Format Converter --> Mixer -> Meter -> Output
```

## Compilation

ForgeAudioD holds a fixed-capacity topology with up to 16 nodes and four input slots per node. The K15.8 control path validates the eight required node kinds, validates required/unused inputs, rejects invalid/self edges, counts exactly eight edges, computes indegrees and performs deterministic Kahn topological sorting. Failure to produce all eight nodes is treated as a cycle/unresolved-dependency failure.

Compilation happens before block execution. The compiled order is immutable during the K15.8 processing proof.

## Processing

The processing path uses only statically reserved buffers and bounded loops. It performs no allocation, locking, sleeping, syscalls, logging or topology compilation.

Four 1,024-byte S16 stereo blocks are generated. Per block:

1. Input copies source PCM into graph storage.
2. Gain applies exact 0.5 gain to the even-valued qualification samples.
3. Splitter creates the two processing branches.
4. Channel Mapper swaps stereo L/R.
5. Format Converter performs S16 stereo interleaved -> planar -> interleaved conversion.
6. Mixer performs signed saturating sample addition.
7. Meter computes peak and sum-of-absolute magnitudes while forwarding PCM.
8. Output publishes the final block.

Every output sample is checked. All eight node kinds must execute exactly four times. The four blocks total 1,024 stereo frames. Final Meter evidence must be `peak=400` and `sum_abs=204800`.

## Runtime ordering

K15.8 executes after K15.7 has completed its real command queues, PCM ring laps, dead-client generation invalidation/reap and persistent heartbeat. It executes **before** the final K15.7 `ServerQualify` aggregate closure. Therefore the kernel cannot intentionally halt a successful qualification before the K15.8 evidence is emitted.

## Scope boundary

K15.8 does not implement K15.9 sample-accurate graph generation switching or K15.11 production resampling/fractional drift correction.
