# Titanweave K15.4 ForgeAudio Real HDA Hardware Backend — Runtime Qualification

Status: **QUALIFIED / FROZEN**

Date: 2026-08-10

Fedora/QEMU runtime qualification PASSED.

Qualified evidence:

- K15.1 real-time execution foundation retained
- K15.2 ForgeAudio kernel ABI retained
- K15.3 audio DMA transport retained
- real PCI HDA controller discovery
- HDA controller reset
- peer DMA coexistence preserved through VT-d
- exact HDA translated DMA domain armed and revoked
- CORB command transport
- RIRB response transport
- codec and widget discovery
- HDA BDL programming
- PCI MSI routing
- real HDA stream interrupt completion
- two playback periods completed
- two capture periods completed
- 2048 playback frames
- 2048 capture frames
- capture DMA changed real memory
- HDA bus mastering revoked after qualification
- ForgeAudio HDA device registered
- playback and capture endpoints registered
- VirtIO-GPU remained operational after HDA qualification
- K14.C32 remained qualified
- stable userspace handoff retained
- intentional Titanweave HALT reached
- raw QEMU exit status 0

Required qualification truth:

- fake_hw=false
- physical_silicon=false

QEMU qualifies Titanweave against the real QEMU HDA hardware model and
Titanweave's PCI/MMIO/DMA/MSI/interrupt paths. It does not claim physical
motherboard audio silicon qualification.

K15.4 is QUALIFIED / FROZEN.

K15.5 — PCM Format Engine is now unlocked.
