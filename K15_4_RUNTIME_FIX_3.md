# K15.4 Runtime Fix 3 — Preserve Frozen GPU DMA During HDA VT-d Window

Observed runtime symptom after HDA MSI qualification progressed: `[FAIL] K6 user runtime reported failure`.

The K6 marker is downstream of K15.4 and can only report either a userspace fault or a failed DISPLAYD recovery qualification. The K15.4 temporary VT-d helper previously replaced the live root table with an HDA-only context and temporarily disabled unrelated PCI bus masters. Because K13 VirtIO-GPU is already live before K15.4, that policy could disturb the frozen graphics transport before K6/DISPLAYD exercised recovery.

Fix:

- require Intel VT-d ECAP.PT support for the coexistence window;
- give every already-active non-HDA bus master on the qualified HDA bus a legacy context-entry with Translation Type `10b` (pass-through);
- keep their PCI bus-master bits untouched for the entire HDA qualification window;
- keep HDA itself behind the translated second-level IOVA domain;
- verify peer bus-master state is unchanged after translation is revoked;
- perform a bounded post-HDA VirtIO-GPU suspend/rearm health probe before continuing boot;
- add `[K6DIAG]` detail if K6 still fails, without weakening the existing K6 failure gate.

This preserves the pre-K15.4 DMA authority of already-live devices while keeping HDA DMA translated and scoped. No K14/K15.1-K15.3 success criterion is weakened.
