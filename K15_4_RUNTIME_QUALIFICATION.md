# Titanweave K15.4 ForgeAudio Real HDA Hardware Backend — Runtime Qualification

Status: **QUALIFIED / FROZEN**

Date: 2026-08-10

Fedora/QEMU runtime qualification PASSED.

Qualified evidence retained by K15.5 baseline:

- K15.1 real-time execution foundation passed;
- K15.2 ForgeAudio kernel ABI passed;
- K15.3 audio DMA transport passed;
- real PCI HDA controller discovery/reset passed;
- peer-DMA coexistence through VT-d passed;
- exact HDA requester translated domain armed and revoked;
- CORB/RIRB codec transport passed;
- codec/widget discovery passed;
- HDA BDL programming passed;
- MSI routing and real HDA stream interrupt completion passed;
- two playback and two capture periods completed;
- 2048 playback and 2048 capture frames completed;
- capture DMA changed real mapped memory;
- bus mastering was revoked after qualification;
- one ForgeAudio HDA device and playback/capture endpoints registered;
- VirtIO-GPU remained operational after HDA qualification;
- K14.C32 remained qualified through intentional HALT;
- raw QEMU exit status was 0.

Qualification truth: `fake_hw=false`, `physical_silicon=false`.

K15.4 is frozen. K15.5 — PCM Format Engine is unlocked.
