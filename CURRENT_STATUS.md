# Titanweave OS — Current Status

**Updated: 2026-08-10**

## Current project state

- **K14 Native Radeon Foundation: COMPLETE / QUALIFIED / FROZEN ✅**
- **K15.1 ForgeAudio RT Foundation: QUALIFIED / FROZEN ✅**
- **K15.2 ForgeAudio Kernel ABI: QUALIFIED / FROZEN ✅**
- **K15.3 ForgeAudio Audio DMA Transport: QUALIFIED / FROZEN ✅**
- **K15.4 Real HDA Hardware Backend: QUALIFIED / FROZEN ✅**
- **K15.5 PCM Format Engine: NEXT / UNLOCKED 🔊**

## Active K15.4 qualification

K15.4 consumes the frozen K15.3 cyclic DMA transport and adds the real HDA hardware-model execution path:

- PCI HDA class/subclass discovery and ForgeBus ownership;
- BAR0 MMIO mapping and controller reset;
- CORB/RIRB DMA command transport;
- codec, audio-function-group, converter and widget discovery;
- HDA BDL and stream descriptor programming;
- exact-requester translated Intel VT-d data-DMA window;
- PCI MSI routed through Titanweave's interrupt router;
- HDA stream interrupt evidence before K15.3 period retirement;
- playback and capture DMA proof;
- capture-memory mutation proof;
- real ForgeAudio HDA device plus playback/capture endpoint registration;
- explicit `fake_hw=false` and `physical_silicon=false` QEMU semantics.

K15.4 deliberately qualifies only 48 kHz / signed 16-bit / stereo. General format/rate/channel negotiation belongs to K15.5.

## ForgeAudio stone contract

```text
K15.1   Real-Time Audio Execution Foundation       FROZEN ✅
K15.2   ForgeAudio Kernel ABI                      FROZEN ✅
K15.3   Audio DMA Transport                        FROZEN ✅
K15.4   Real HDA Hardware Backend                  FROZEN ✅
K15.5   PCM Format Engine                          NEXT 🔊
K15.6   ForgeAudioD                                LOCKED
K15.7   Lock-Free Audio Transport                  LOCKED
K15.8   ForgeAudio Graph Engine                    LOCKED
K15.9   Sample-Accurate Graph Switching            LOCKED
K15.10  Clock & Synchronization Engine             LOCKED
K15.11  Production Resampler                       LOCKED
K15.12  Full-Duplex Capture + Playback             LOCKED
K15.13  Routing, Per-App Mixing & Monitor Paths    LOCKED
K15.14  Latency & XRUN Engine                      LOCKED
K15.15  Fault Recovery & Hotplug                   LOCKED
K15.16  Full Production Qualification + Freeze     LOCKED
```

## Qualification boundary

QEMU HDA is real execution against QEMU's emulated HDA controller/codec model and real Titanweave PCI/MMIO/DMA/MSI code paths. It is not physical motherboard/audio-codec silicon. `physical_silicon=false` is therefore required for the QEMU K15.4 gate.

See `README.md`, `PROJECT_VISION.md`, `K15_STATUS.md`, and `K15_STONE_CONTRACT.md`.
