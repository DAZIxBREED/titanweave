# Titanweave K14.C11 — Reviewed Radeon register definitions + IP-base resolver gate

K14.C11 is forked from the frozen, QEMU-qualified K14.C10 baseline.

## Purpose

C10 proved a bounded read executor but intentionally left all physical Radeon offset tables empty. C11 adds the first source-reviewed register definitions for Titanweave's initial Radeon targets while correcting a subtle but critical addressing issue: AMD generated register headers express many register locations as **IP-relative DWORD register indices**, not raw BAR-relative byte offsets.

Treating `0x0da4` as `BAR + 0x0da4` would therefore be an unsafe assumption. C11 adds an explicit `IpBaseMap` and `resolve_byte_offset()` stage. The resolver converts `IP base in DWORDs + register index` to a byte offset only after the IP base has been obtained from trusted AMD IP-discovery data.

## Reviewed definitions

For Navi 21 / GC 10.3, C11 records `GRBM_STATUS=0x0da4`, `GRBM_CHIP_REVISION=0x0dc1`, and `SDMA0_STATUS_REG=0x0025`. For Navi 48 / GC 12.0, C11 records `GRBM_STATUS=0x0da4`, `GRBM_CHIP_REVISION=0x0dc1`, and `SDMA0_STATUS_REG=0x0024`.

These values are grounded in the AMD-authored generated register headers carried by upstream Linux (`gc_10_3_0_offset.h` and `gc_12_0_0_offset.h`). The upstream AMDGPU ABI also documents support for reading status-register classes such as GRBM and SDMA.

## Safety boundary

C11 does **not** fabricate GC or SDMA IP bases. The runtime `IpBaseMap` is deliberately empty until Titanweave implements a trusted IP-discovery parser. Therefore C11 qualifies reviewed definitions and address resolution logic in QEMU but performs zero physical MMIO reads.

Still fenced: register writes, firmware upload, command submission, Radeon bus mastering.

## Next

K14.C12 should parse/verify the AMD IP-discovery base map for the exact ASIC, resolve these C11 register indices into aperture byte addresses, bounds-check them, and then permit the first genuine status reads.
