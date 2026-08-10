# K15.4 Tester Guide — ForgeAudio Real HDA Hardware Backend

## Run

```bash
cd ~/Downloads/titanweave-kernel-k15-4-integrated
chmod +x tools/run-k15-4-qemu-forgeaudio-hda.sh
./tools/validate-source.sh
K15_DISPLAY=none ./tools/run-k15-4-qemu-forgeaudio-hda.sh
```

The K15.4 QEMU runner adds a real QEMU ICH9 HDA controller with MSI and an HDA duplex codec. It uses the null host-audio backend so hardware DMA, codec and IRQ behavior are exercised without requiring host speakers/microphone access.

## Required evidence

The checker requires:

- inherited K15.1, K15.2 and K15.3 runtime evidence;
- real HDA PCI discovery and controller reset;
- at least one codec after reset;
- live CORB and RIRB DMA command transport;
- audio function-group/widget discovery;
- playback and capture converters;
- exact-requester translated VT-d DMA mapping;
- HDA bus mastering only inside that translated window;
- real BDL/stream descriptor programming;
- PCI MSI enabled through Titanweave's interrupt router;
- at least four HDA stream interrupts;
- exactly two playback periods / 2048 playback frames;
- exactly two capture periods / 2048 capture frames;
- capture memory modified by the device DMA path;
- HDA bus mastering disabled after domain revocation;
- one real ForgeAudio HDA device with two endpoints;
- `fake_hw=false`;
- `physical_silicon=false` on QEMU;
- K14.C32 still reaches intentional HALT;
- no `[FAIL]` line.

The host should finish with:

```text
Titanweave K15.4 ForgeAudio real HDA hardware backend runtime qualification PASSED.
```

Do not start K15.5 if this gate fails.
