# Titanweave K15 ForgeAudio Stone Contract

Status: **LOCKED**

K15 is ForgeAudio. K15 contains **exactly 16 gates**. The scope may be refined inside a gate to fix defects, but it may not expand into K15.17+ or split into an unbounded chain. A gate is complete only when its required implementation is real, source-validated, and runtime-qualified. Required execution paths may not contain stubs, fake success, fake DMA/IRQ completion, placeholder devices, or TODO implementations.

## Non-negotiable rules

- No stubs or placeholder success paths on a required K15 execution path.
- No fake hardware qualification. QEMU-safe deferral is valid only when the real hardware path exists and explicitly reports that physical execution is unavailable.
- Audio/RT execution may not allocate, perform filesystem I/O, sleep on an unbounded wait, or take an unbounded mutex from the real-time path.
- Every bounded resource has an explicit capacity and failure result.
- Every gate preserves the qualified K1-K14 baseline.
- Every gate receives source validation and runtime qualification before the next gate is frozen.
- K15 ends at K15.16.

## The 16 gates

1. **K15.1 — Real-Time Audio Execution Foundation**  
   RT scheduling class, bounded execution budget, CPU affinity/reservation, priority inheritance, deadline/period tracking, and bounded preemption protection.

2. **K15.2 — ForgeAudio Kernel ABI**  
   Versioned kernel/userspace audio objects and lifecycle: device, endpoint, stream, buffer, clock, events and fences.

3. **K15.3 — Audio DMA Transport**  
   Cyclic DMA, period completion, position tracking, IOMMU isolation, buffer ownership and XRUN detection.

4. **K15.4 — Real HDA Hardware Backend**  
   PCI HDA controller, CORB/RIRB, codec/widget discovery, BDL/stream descriptors, interrupts, playback and capture.

5. **K15.5 — PCM Format Engine**  
   Canonical PCM formats, interleaved/planar conversion, channel mapping and supported-rate negotiation.

6. **K15.6 — ForgeAudioD**  
   Real userspace audio server owning devices, streams, graph control, routing, clocks, buffers, telemetry and recovery.

7. **K15.7 — Lock-Free Audio Transport**  
   Bounded application/server audio rings and command queues with sequence/generation tracking and dead-client isolation.

8. **K15.8 — ForgeAudio Graph Engine**  
   Operational Input, Output, Gain, Mixer, Splitter, Channel Mapper, Format Converter and Meter nodes.

9. **K15.9 — Sample-Accurate Graph Switching**  
   Exact-frame graph generation activation, stale-buffer rejection and bounded control-plane switching.

10. **K15.10 — Clock & Synchronization Engine**  
    Device, monotonic, stream, presentation and capture clocks with measurable cross-device drift.

11. **K15.11 — Production Resampler**  
    Bounded real-time sample-rate conversion and fractional drift correction.

12. **K15.12 — Full-Duplex Capture + Playback**  
    Concurrent input/output with independent clock handling and correct graph transport.

13. **K15.13 — Routing, Per-App Mixing & Monitor Paths**  
    Per-stream volume/mute/solo, buses, monitoring, routing and channel assignment.

14. **K15.14 — Latency & XRUN Engine**  
    Input/output/graph latency, callback duration, deadline misses, underruns, overruns and dropped-frame telemetry.

15. **K15.15 — Fault Recovery & Hotplug**  
    Device removal/reconnect, endpoint rebinding, DMA shutdown, client notification, clock re-election and safe restart.

16. **K15.16 — Full Production Qualification + Freeze**  
    Sustained playback/capture/full-duplex, multi-client routing, graph edits, resampling, drift, hotplug, XRUN recovery and repeated lifecycle qualification. Passing K15.16 freezes ForgeAudio K15.
