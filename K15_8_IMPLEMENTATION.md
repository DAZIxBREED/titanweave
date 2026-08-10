# K15.8 — ForgeAudio Graph Engine

K15.8 is gate 8 of the locked 16-gate ForgeAudio stone contract. It consumes qualified/frozen K15.7 lock-free transport and adds the first operational ForgeAudio processing graph.

## Scope

This gate implements exactly the eight nodes required by the stone contract:

- Input
- Output
- Gain
- Mixer
- Splitter
- Channel Mapper
- Format Converter
- Meter

K15.8 does **not** implement sample-accurate graph switching (K15.9), the clock/synchronization engine (K15.10), or production resampling/fractional drift correction (K15.11).

## Graph model

`kernel/weavecore/src/forgeaudio_graph_engine.rs` provides a fixed-capacity graph with at most 16 nodes and four input edges per node. Topology construction and validation are control-path operations. Compilation performs deterministic topological ordering and rejects missing inputs, extra inputs, invalid node references, duplicate connections and cycles.

Once compiled, topology mutation is rejected. The processing path uses only fixed arrays and bounded loops. It does not allocate, take a Mutex/SpinLock, perform filesystem I/O, or sleep.

## Operational node qualification

The qualification graph contains exactly eight nodes and eight edges:

```text
Input -> Gain -> Splitter -> Channel Mapper --\
                         \-> Format Converter --> Mixer -> Meter -> Output
```

The Gain node applies Q15 0.5 gain. Splitter creates two graph paths. Channel Mapper swaps stereo left/right. Format Converter executes an S16 stereo interleaved -> planar -> interleaved round trip. Mixer performs saturating sample addition. Meter measures peak and sum-of-absolute sample magnitude while passing PCM onward. Output emits the resulting S16 block.

Four 1,024-byte S16 stereo blocks are processed. Every node must execute exactly four times. The four blocks represent 1,024 stereo frames in total. Qualification verifies every output sample, four format-layout round trips, final meter peak 400 and final meter absolute sum 204,800.

## Runtime boundary

K15.8 executes only after the inherited K15.7 ForgeAudioD lock-free transport qualification succeeds. A graph validation or processing failure fails closed and prevents the normal qualified userspace halt.

A successful runtime emits:

```text
[K15GR] graph compiled: ... nodes=8 edges=8 ...
[K15GR] node execution: Input=4 Output=4 Gain=4 Mixer=4 Splitter=4 ChannelMapper=4 FormatConverter=4 Meter=4
[K15GR] PCM verified: blocks=4 frames=1024 ... output_verified=true ...
[K15OK] K15.8 ForgeAudio Graph Engine qualified: ...
[K15GR] ForgeAudio Graph Engine ready: ... sample_accurate_switching=false resampling=false
[USER] [forgeaudiod] K15.8 graph engine ready: ...
```

K15.9 remains locked until Fedora/QEMU runtime qualification passes.
