# K15.5 Runtime Fix 1 — Negative Capability Vector + Inherited Checker Ordering

The first Fedora/QEMU K15.5 run correctly exposed two qualification-harness defects.

1. The synthetic capability set advertised HDA rate bit 10 (192 kHz), while the negative exact-rate test incorrectly expected 192 kHz to be rejected. The test now uses 176.4 kHz (bit 9), which is deliberately absent, and first proves the unsupported premise before calling negotiation.
2. The frozen K15.4 `[K15CO]` HDA/GPU coexistence proof was emitted after the K15.5 self-test. A K15.5 failure therefore halted before the inherited K15.4 marker could be emitted. The coexistence proof now runs immediately after K15.4 HDA readiness and before K15.5 begins.
3. Standalone K15.3/K15.4 checkers remain globally strict about `[FAIL]`. When reused by K15.5 solely for inherited evidence, an explicit environment flag suppresses only their redundant global failure scan; the K15.5 checker still performs the final strict global `[FAIL]` scan.

No PCM negotiation policy was weakened: 192 kHz remains valid whenever a device advertises it.
