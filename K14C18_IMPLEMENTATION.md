# Titanweave K14.C18 — AMD Discovery Snapshot Verification Gate

K14.C18 extends the frozen K14.C17 AMD IP-discovery parser with the integrity
and acquisition contract needed before Titanweave can trust a physical Radeon
IP-discovery snapshot.

## Scope

C18 adds:

- source-backed AMD discovery TMR constants and scratch-register contract;
- a bounded wrapping-16-bit byte-sum checksum implementation;
- binary checksum verification over the payload beginning immediately after
  `binary_checksum`, matching upstream AMDGPU;
- IP_DISCOVERY table checksum verification using the table checksum from the
  binary table list and the table's own declared size;
- a synthetic checksum-qualified discovery snapshot self-test;
- userspace query/diagnostic integration and QEMU qualification markers;
- fail-closed physical acquisition state.

## Upstream source contract

Reviewed against current upstream Linux AMDGPU sources:

- `drivers/gpu/drm/amd/include/discovery.h`
  - binary and IP-discovery signatures;
  - `binary_checksum`, `binary_size`, and table-list layout.
- `drivers/gpu/drm/amd/amdgpu/amdgpu_discovery.h`
  - `DISCOVERY_TMR_SIZE = 10 << 10`;
  - `DISCOVERY_TMR_OFFSET = 64 << 10`.
- `drivers/gpu/drm/amd/amdgpu/amdgpu_discovery.c`
  - TMR discovery through `mmDRIVER_SCRATCH_0..2` when available;
  - memory/VRAM snapshot acquisition flow;
  - wrapping 16-bit sum-of-bytes checksum algorithm;
  - binary checksum range begins after `binary_checksum`;
  - individual table checksum verification.

Titanweave imports the semantics, not Linux driver code.

## Safety boundary

C18 intentionally keeps these capabilities OFF:

- live discovery TMR read;
- live Radeon VRAM read;
- Radeon MMIO writes;
- firmware upload;
- GPU command submission;
- Radeon bus-master enable.

A physical Radeon therefore remains in a diagnostic/fallback state. C18 makes
snapshot integrity verifiable but does not claim that Titanweave has fetched a
physical discovery blob yet.

## Promotion rule

A later milestone may promote physical acquisition only after it provides a
bounded source-backed mapping/read path for the discovery TMR and proves that:

1. the exact Radeon identity still matches the verified profile;
2. the snapshot size is bounded;
3. the binary signature/parser succeeds;
4. the binary checksum passes;
5. the IP_DISCOVERY table checksum passes;
6. no write, firmware, command submission, or bus-master capability is needed.
