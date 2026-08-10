# K15.4 Runtime Fix 1 — HDA CORB/RIRB command-ring progress

Observed Fedora/QEMU failure:

```text
intel-hda: write to r/o reg CORBSIZE
intel-hda: write to r/o reg RIRBSIZE
[FAIL] K15.4 ForgeAudio real HDA hardware backend qualification failed: HDA CORB command timed out waiting for RIRB response
```

The host CPU vendor is unrelated. QEMU presents an emulated ICH9 HDA PCI function (`8086:293e`) to Titanweave.

Fixes in this revision:

- stop writing `CORBSIZE`/`RIRBSIZE`; consume the controller-selected ring geometry and validate it against capability bits;
- keep distinct CORB and RIRB entry counts;
- implement the required two-phase `CORBRP` reset/readback handshake;
- honor write-only `RIRBWP` reset semantics and verify the resulting write pointer is zero;
- verify command-ring DMA base-address and `RINTCNT` programming;
- acknowledge `RIRBSTS` after synchronously consumed RIRB responses, so a response-count/empty-ring interrupt condition cannot stall the next CORB verb while MSI delivery is deferred;
- distinguish CORB DMA-fetch failure from a fetched command that failed to produce a RIRB response;
- emit complete CORB/RIRB register diagnostics on timeout.

No K15.5 functionality is introduced. K15.1-K15.3 remain frozen.
