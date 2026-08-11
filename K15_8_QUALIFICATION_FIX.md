# K15.8 Qualification Checker Fix

The first Fedora/QEMU K15.8 run executed and verified the ForgeAudio graph successfully, but the serial checker rejected the run for three harness-only reasons:

1. It required `compile_outside_rt=true` in one summary line even though the stronger runtime-ready proof already checks `topology_compile_rt=false` and the source test verifies compilation is outside the block executor.
2. It required the redundant summary label `all_nodes=true` even though the checker individually verifies Input, Output, Gain, Mixer, Splitter, ChannelMapper, FormatConverter and Meter each executed exactly four times.
3. It expected K15.8's userspace proof before K15.7's kernel ServerQualify closure. The actual userspace-owned architecture necessarily returns K15.7 qualification to ForgeAudioD first, then emits K15.8 proof, then reaches final QUAL/HALT.

The corrected ordering requirement is:

`K15.7 qualification -> K15.8 graph qualification -> final QUAL -> HALT`

No kernel, ABI, transport, audio-client, ForgeAudioD graph, or DSP runtime code was changed by this correction.
