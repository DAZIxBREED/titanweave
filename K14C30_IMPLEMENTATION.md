# Titanweave K14.C30 — Complete Basic Display Engine

K14.C30 is the locked-roadmap display milestone built on frozen K14.C29. It is intentionally large: one milestone closes mode parsing/selection, connector/CRTC/plane ownership, scanout buffering, present/page-flip behavior, atomic mode state, hotplug bookkeeping, userspace visibility, and the reviewed DCN401 capability boundary.

## Operational backend

C30 uses the framebuffer handed off by UEFI GOP as a real active scanout backend. It allocates two reclaimable C28 GTT objects sized to the active framebuffer, pins them for display ownership, renders distinct frames, presents each into the live framebuffer with volatile writes, and verifies that the live scanout fingerprint changes. This is a real basic display path in QEMU and on firmware-provided linear framebuffers; it is not described as native DCN programming.

## Mode and connector engine

- Validates EDID base headers and checksums.
- Extracts bounded detailed timings and computes refresh rates.
- Deterministically selects a requested 2560x1440 timing when present, otherwise preferred/first detailed timing.
- Owns a bounded four-connector model with explicit connector, CRTC and plane assignment.
- Commits the active firmware mode atomically.
- Rejects unsupported post-ExitBootServices GOP mode changes and proves state rollback.
- Maintains a bounded hotplug event journal with monotonic sequence numbers.

## DCN401 reviewed boundary

C30 records source-reviewed DCN 4.01 resource counts (four timing generators, four video planes, four stream encoders, four DDC engines and associated display resources) but performs no unreviewed DCN register writes. Physical HPD and native DCN modesetting remain false until the exact register/transmitter path is reviewed and its physical prerequisites are live.

## Still forbidden

- No arbitrary display MMIO.
- No caller-selected display register/value writes.
- No native DCN programming claim.
- No physical HPD enable claim.
- No placeholder or stub subsystem.

The next locked milestone is K14.C31 graphics + compute execution.
