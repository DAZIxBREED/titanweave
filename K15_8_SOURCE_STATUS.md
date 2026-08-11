# Titanweave K15.8 ForgeAudio Graph Engine — Source Status

Status: **SOURCE-INTEGRATED / FEDORA-QEMU RUNTIME QUALIFICATION PENDING**

Baseline: **user-confirmed qualified K15.7 integrated source tree**.

Implemented:

- userspace-owned ForgeAudioD graph engine;
- byte-for-byte preservation checks for the qualified scheduler, kernel entry, process runtime, syscall layer, K15.7 transport, ForgeAudio ABI, audio client and userspace ABI include;
- fixed 16-node / four-input graph capacity;
- required eight-node K15.8 DAG;
- deterministic topological compilation and cycle/unresolved-dependency fail-closed behavior;
- Input, Output, Gain, Mixer, Splitter, Channel Mapper, Format Converter and Meter execution;
- four 1,024-byte S16 stereo blocks / 1,024 frames total;
- exact output verification;
- per-node execution counts;
- four format-layout round trips;
- final Meter peak 400 and absolute-sum 204,800;
- allocation-free, lock-free, syscall-free processing path;
- graph completion after inherited K15.7 server qualification and before final K14.C32 qualification/halt;
- K15.8 source and QEMU serial qualification tooling.

K15.9 remains locked until this candidate passes Fedora/QEMU runtime qualification.
