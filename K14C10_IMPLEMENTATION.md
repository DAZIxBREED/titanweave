# Titanweave K14.C10 — Per-IP MMIO whitelist engine

C10 forks the qualified C9 tree. It adds the bounded volatile-read executor, descriptor validation, profile-to-whitelist selection, userspace query/reporting, and a strict promotion gate for physical Radeon MMIO reads.

## Safety status

The C10 engine exists and is source-qualified, but the physical Navi 21 and Navi 48 whitelist arrays intentionally remain empty in this cut. Titanweave therefore performs **zero physical Radeon MMIO reads** until exact offsets are reviewed for the corresponding IP revision. The C7 read-only page-table mapping remains the only allowable aperture. MMIO writes, firmware upload, command submission, and Radeon bus mastering remain disabled.

## Promotion rule

A live read requires: C7 read-only aperture + C9 verified profile + consistent live PCI identity + non-empty reviewed whitelist + alignment/bounds/side-effect checks. Any failure keeps the device fenced and leaves VirtIO-GPU/GOP fallback armed.
