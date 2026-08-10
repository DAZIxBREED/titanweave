# K15.4 — ForgeAudio Real HDA Hardware Backend

K15.4 is gate 4 of the locked 16-gate ForgeAudio stone contract. It consumes the frozen K15.3 cyclic DMA transport and turns that backend-neutral transport into a real PCI High Definition Audio backend. This gate does not implement the K15.5 format engine, ForgeAudioD, lock-free client transport, graph processing, resampling, full-duplex policy, routing, or hotplug recovery.

## Implemented

### Real PCI HDA ownership

`kernel/weavecore/src/forgeaudio_hda.rs` discovers a real PCI multimedia/audio controller (`class 0x04`, `subclass 0x03`), claims the exact function through ForgeBus, maps BAR0 through Titanweave's kernel MMIO aperture, keeps bus mastering disabled until a translated DMA domain is live, and performs the HDA controller reset sequence through `GCTL.CRST`.

The backend reads `GCAP`, HDA version, input/output stream counts and `STATESTS`. Qualification fails when no HDA controller, no codec, or no playback/capture stream descriptor exists. No placeholder device is fabricated.

### CORB/RIRB command transport

K15.4 allocates physically backed command pages and maps them into the exact HDA requester domain. It programs the real CORB and RIRB base registers, negotiates a supported command-ring size, resets hardware pointers, enables CORB/RIRB DMA, sends actual codec verbs, and consumes responses from RIRB memory.

The codec path discovers:

- codec address and vendor identity;
- audio function group;
- subordinate widget range;
- first audio-output converter;
- first audio-input converter;
- pin widgets and bounded connection-list relationships where exposed.

The gate then powers the discovered function/converters to D0, configures the converters for the K15.4 qualification PCM mode, binds stream tags, and enables discovered playback/capture pins.

### K15.3 translated DMA integration

K15.4 does not bypass the frozen K15.3 isolation contract. `translated_dma::with_temporary_translated_domain` creates a bounded Intel VT-d second-level domain for the exact HDA requester. The window maps only:

- CORB;
- RIRB;
- playback BDL;
- capture BDL;
- playback audio ring;
- capture audio ring.

Unrelated bus masters are quiesced while the single-requester root table is active. HDA PCI bus mastering is enabled only after every mapping exists and is disabled before the domain is invalidated/revoked. The K15.3 `DmaIsolationLease` is constructed from the real requester/domain/IOVA mapping before `AudioDmaTransport::arm_hardware` succeeds.

The scoped translated-domain lifetime is intentional for K15.4 qualification. A later ForgeAudio server gate owns persistent device lifetime; K15.4 proves the real hardware backend without turning a boot qualification into a hidden long-lived server.

### Real HDA BDL and stream descriptors

Playback and capture each use a real HDA stream descriptor and a real Buffer Descriptor List entry with IOC completion. K15.4 deliberately uses one hardware BDL entry at a time so K15.3's strict one-period device-ownership invariant remains true: software acquires exactly one translated period, publishes that period to HDA, waits for the HDA hardware completion interrupt, stops the stream, and only then retires the K15.3 period.

The QEMU gate completes two playback periods and two capture periods. Playback memory contains deterministic non-zero stereo PCM. Capture memory is poisoned before DMA and must be modified by HDA capture DMA before the period is accepted.

### Real MSI completion

K15.4 allocates an interrupt vector through Titanweave's `InterruptRouter`, installs the HDA handler, programs the PCI MSI capability, enables stream interrupt bits in HDA `INTCTL`, and acknowledges HDA stream status in the actual device interrupt handler.

The K15.3 `backend_complete_period` path is not called until `STREAM_IRQ_EVENTS` proves a real HDA stream interrupt has been dispatched through Titanweave's IDT/ForgeBus interrupt path. Poll-only or synthetic completion cannot satisfy the K15.4 gate.

The QEMU K15.4 runner disables VT-d interrupt remapping for this gate because Titanweave's current generic MSI primitive programs direct APIC MSI messages; data DMA remains fully translated through VT-d. No interrupt-remapping capability is falsely claimed.

### ForgeAudio ABI registration

After the hardware qualification window succeeds, K15.4 registers one real HDA `AudioDeviceInfo` plus two proven endpoints with the frozen K15.2 ABI:

- HDA Playback;
- HDA Capture.

K15.4 advertises only the qualification mode: 48 kHz, signed 16-bit stereo. General format/rate/channel negotiation is exclusively K15.5.

## Required runtime evidence

A passing run emits all of the following families:

```text
[K15HDA] controller:
[IOMA] temporary translated device domain armed:
[K15HDA] command+codec:
[K15HDA] DMA+IRQ:
[IOMV] temporary translated device domain revoked:
[K15HDA] ForgeAudio registry:
[K15OK] K15.4 ForgeAudio real HDA hardware backend qualified:
[K15HR] ForgeAudio HDA ready:
```

The final K15.4 line must prove `CORB=true`, `RIRB=true`, `BDL=true`, `translated_dma=true`, `MSI=true`, `irq=true`, `playback=true`, `capture=true`, `registry=true`, and `fake_hw=false`.

QEMU's HDA device is an emulated hardware model, not physical silicon. Therefore the QEMU qualification explicitly records `physical_silicon=false`. The same HDA backend code is PCI/MMIO based; physical-silicon evidence is separate from the QEMU gate and is never fabricated.

K15.5 — PCM Format Engine remains locked until Fedora/QEMU K15.4 runtime qualification passes.

## Runtime correction: CORB/RIRB progress

The initial Fedora/QEMU run reached the real emulated ICH9 HDA controller and exact-requester VT-d domain but timed out during codec command transport. The corrected backend does not write fixed/read-only `CORBSIZE` or `RIRBSIZE`; it validates the controller-selected geometry. It also uses the specified two-phase `CORBRP` reset handshake and acknowledges `RIRBSTS` after each synchronously consumed response. This acknowledgement is required to let controllers whose response threshold has fired accept subsequent CORB work even when the corresponding MSI has not yet been serviced by the CPU. Detailed pointer/status diagnostics are emitted if a later command still stalls.


### Runtime fix 2 — bounded interrupt-enable window

The second Fedora/QEMU run proved real HDA stream DMA progress and IOC completion (`SDnSTS=0x24`, FIFO-ready + buffer-completion) but observed zero Titanweave stream IRQ dispatches. The root cause was CPU interrupt state: K15.1 intentionally restores boot to `IF=0` after its RT scheduler qualification, while K15.4 originally busy-waited for an MSI without temporarily re-enabling local interrupts. K15.4 now opens a bounded local interrupt window only while waiting for each hardware stream completion, restores the exact prior IF state before continuing initialization, verifies HDA global+stream `INTCTL` enablement before RUN, and includes `INTCTL`, `INTSTS`, stream bit and IF state in timeout diagnostics. No poll-only completion is accepted; `backend_complete_period` still requires the real HDA MSI handler to advance `STREAM_IRQ_EVENTS`.
