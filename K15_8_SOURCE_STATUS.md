# Titanweave K15.8 ForgeAudio Graph Engine — Source Status

Status: **SOURCE CORE + QUALIFICATION HARNESS INTEGRATED / FEDORA WIRING + RUNTIME QUALIFICATION PENDING**

Baseline: qualified K15.7 ForgeAudio lock-free transport.

Implemented in this gate:

- fixed-capacity 16-node / four-input-per-node graph storage;
- deterministic topological compilation with cycle rejection;
- immutable topology after compile;
- operational Input, Output, Gain, Mixer, Splitter, Channel Mapper, Format Converter and Meter nodes;
- allocation-free, lock-free bounded processing path;
- four-block / 1,024-frame S16 stereo runtime proof;
- exact output verification;
- per-node execution counters;
- Meter peak and absolute-sum verification;
- explicit exclusion of K15.9 sample-accurate switching;
- explicit exclusion of K15.11 production resampling;
- K15.8 source checker, serial checker and QEMU wrapper;
- exact-anchor integration patcher for the frozen K15.7 tree.

The GitHub connector can safely add the new K15.8 files but only performs whole-file replacement for existing large files. `tools/apply-k15-8-integration.py` therefore performs the remaining surgical wiring locally against the exact K15.7 baseline before Fedora source/runtime qualification.

K15.9 remains locked until K15.8 passes Fedora/QEMU runtime qualification.
