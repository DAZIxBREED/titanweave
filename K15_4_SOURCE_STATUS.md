# Titanweave K15.4 ForgeAudio Real HDA Hardware Backend — Source Status

Status: **QUALIFIED / FROZEN**

Baseline: qualified/frozen K15.3.

Implemented in this gate:

- exact PCI HDA class/subclass discovery and ForgeBus ownership;
- HDA BAR0 MMIO mapping and controller reset;
- GCAP/version/stream-capability verification;
- real CORB/RIRB DMA setup and codec verb transport;
- codec, audio-function-group and widget discovery;
- bounded converter/pin connection discovery;
- playback and capture converter configuration for the K15.4 proof mode;
- real HDA stream descriptor and BDL programming;
- exact K15.3 playback/capture DMA transport hardware arming;
- exact-requester Intel VT-d temporary translated DMA window;
- unrelated-bus-master quiesce and mandatory HDA DMA revocation;
- PCI MSI programming through Titanweave's interrupt router;
- hardware stream-interrupt accounting before transport completion;
- two real playback and two real capture period completions;
- capture-memory mutation verification;
- real HDA device + playback/capture endpoint registration in ForgeAudio ABI v1;
- explicit `fake_hw=false` and `physical_silicon=false` QEMU semantics;
- K15.4 source and runtime qualification tools.

No K15.5 PCM format engine work is included. The qualification format is intentionally fixed at 48 kHz / signed 16-bit / stereo.

Fedora/QEMU runtime qualification PASSED on 2026-08-10. K15.4 is frozen and K15.5 — PCM Format Engine is unlocked.

### Runtime fix 1

The first Fedora/QEMU HDA run proved PCI discovery, controller reset, codec presence and exact-requester VT-d arming, then exposed a CORB/RIRB command-progress bug. The backend now consumes fixed/read-only controller-selected `CORBSIZE`/`RIRBSIZE`, performs the required `CORBRP` reset handshake, validates base/pointer programming, and explicitly acknowledges synchronously consumed `RIRBSTS` responses so the next CORB verb cannot be blocked behind a deferred MSI response-count condition. Timeout diagnostics now separate CORB DMA-fetch failure from RIRB-response failure.


### Runtime fix 2 — bounded interrupt-enable window

The second Fedora/QEMU run proved real HDA stream DMA progress and IOC completion (`SDnSTS=0x24`, FIFO-ready + buffer-completion) but observed zero Titanweave stream IRQ dispatches. The root cause was CPU interrupt state: K15.1 intentionally restores boot to `IF=0` after its RT scheduler qualification, while K15.4 originally busy-waited for an MSI without temporarily re-enabling local interrupts. K15.4 now opens a bounded local interrupt window only while waiting for each hardware stream completion, restores the exact prior IF state before continuing initialization, verifies HDA global+stream `INTCTL` enablement before RUN, and includes `INTCTL`, `INTSTS`, stream bit and IF state in timeout diagnostics. No poll-only completion is accepted; `backend_complete_period` still requires the real HDA MSI handler to advance `STREAM_IRQ_EVENTS`.

## Runtime fix 3

Preserve already-live peer DMA during the scoped HDA VT-d qualification window using Intel VT-d pass-through contexts, verify peer bus-master state after revoke, and verify frozen VirtIO-GPU health before userspace. K6 now emits detailed failure diagnostics without weakening the gate.
