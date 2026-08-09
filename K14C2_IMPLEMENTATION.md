# K14.C2 Implementation Record — Persistent Domain + AMD Bring-up Contract

K14.C2 forks from the frozen, QEMU-qualified K14.C1 baseline.

## Added
- Real VT-d persistent-domain surrogate using QEMU EDU: one requester/context and two IOVAs are retained across three DMA epochs before explicit revocation.
- AMD-first ASIC identity handoff from the C1 ForgeBus binding state.
- Firmware bring-up plan covering VBIOS, PSP, SMU, GC, SDMA, and scheduler/MES-family firmware roles.
- Command-ring bootstrap geometry and strict lifecycle ordering.
- DISPLAYD C2 status query.
- Fail-closed rule: a QEMU surrogate can never promote the actual Radeon domain bit.

## Deliberate boundary
QEMU has no native Radeon. K14.C2 therefore proves persistent translated-domain mechanics with EDU while the real AMD requester remains fenced. Vendor MMIO writes, firmware upload, native bus mastering, and Radeon command submission remain disabled until the actual GPU RID is bound to a hardware-translated domain on bare metal.
