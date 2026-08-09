# Titanweave K14.C18 — AMD Discovery Snapshot Verification Gate

K14.C18 extends the frozen K14.C17 AMD IP-discovery parser with source-backed integrity verification and the acquisition contract needed before Titanweave can trust a physical Radeon discovery snapshot.

## Added in C18

- bounded wrapping 16-bit byte-sum checksum engine matching AMDGPU semantics;
- binary checksum verification beginning immediately after the `binary_checksum` field;
- IP_DISCOVERY table checksum verification;
- AMD discovery TMR constants and scratch-register acquisition contract;
- synthetic checksum-qualified discovery snapshot self-test;
- fail-closed runtime and userspace status path.

## Safety boundary

C18 keeps physical TMR/VRAM reads, Radeon MMIO writes, firmware upload, GPU command submission, and Radeon bus-master enable OFF. QEMU therefore validates the parser/checksum/runtime contract while physical snapshot acquisition remains deferred.

The complete integrated milestone source and Fedora/QEMU qualification package is distributed separately as the K14.C18 source ZIP.