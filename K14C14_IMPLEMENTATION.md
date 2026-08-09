# K14.C14 — Controlled Radeon Write-Promotion Readiness

K14.C14 is based on the frozen, QEMU-qualified K14.C13 tree.

## Purpose

C14 does **not** perform the first Radeon write. It defines and qualifies the complete prerequisite gate that a later milestone must satisfy before any write-side capability may be considered.

Eligibility requires:

- exact verified AMD profile and consistent live PCI identity;
- the persistent translated DMA-domain state from C6;
- trusted GC/SDMA base provenance from C12;
- the modern Radeon BAR5 register aperture;
- the complete physical read proof and nonzero proof fingerprint from C13;
- a fresh PCI command-register check proving bus mastering remains disabled;
- no pending Navi48 trusted-IP-discovery dependency.

Even when every prerequisite is true, C14 leaves promotion disabled.

## Hard fences

- MMIO writes: OFF
- firmware upload: OFF
- command submission: OFF
- Radeon bus mastering: OFF
- write-promotion switch: OFF

QEMU has no physical Radeon and therefore must report the gate as deferred while retaining the K13 VirtIO/GOP fallback.

## New serial markers

- `[C14PG]` promotion policy
- `[C14CK]` physical prerequisite check
- `[C14HW]` physical readiness state
- `[C14RD]` C14 state summary
- `[C14OK]` successful initialization
