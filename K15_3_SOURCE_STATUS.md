# Titanweave K15.3 ForgeAudio Audio DMA Transport — Source Status

Status: **QUALIFIED / FROZEN**

Baseline: qualified/frozen K15.2.

Fedora/QEMU runtime qualification passed on 2026-08-09. K15.3 is frozen and K15.4 is unlocked.

Implemented in this gate:

- real physically contiguous DMA-ring allocation;
- supervisor-only kernel DMA mapping and explicit teardown;
- bounded cyclic ring with up to 32 periods / 1 MiB;
- exact playback/capture period ownership transitions;
- period queue/acquire/complete/release operations;
- cumulative frame/byte position and ring-wrap tracking;
- strict single in-flight period enforcement;
- translated-IOMMU lease requirement before production backend access;
- IOVA device-address generation after hardware arm;
- playback underrun and capture overrun detection/counters;
- deterministic real-memory transport self-test;
- explicit no-audio-hardware / no-fake-DMA qualification semantics;
- K15.3 source and serial qualification tools.

K15.3 is frozen. K15.4 — Real HDA Hardware Backend is the active gate.
