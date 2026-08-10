# K15.3 Freeze — ForgeAudio Audio DMA Transport

**Status: QUALIFIED / FROZEN**

K15.3 passed Fedora/QEMU runtime qualification on 2026-08-09 and is frozen as the baseline for K15.4.

Frozen evidence:
- inherited K15.1 RT and K15.2 ABI qualification retained;
- translated DMA platform proof retained;
- physically backed cyclic transport self-test completed 12 periods / 3 wraps / 1536 frames;
- playback/capture ownership transitions passed;
- translated-IOMMU hardware arm is fail-closed and raw arm was rejected;
- playback underrun and capture overrun detection each fired exactly once in the bounded test;
- QEMU HDA remained honestly deferred and `fake_dma=false`;
- K14.C32 production/stability qualification remained intact through intentional HALT.

K15.4 — Real HDA Hardware Backend is unlocked. K15 remains bound by the 16-gate ForgeAudio stone contract.
