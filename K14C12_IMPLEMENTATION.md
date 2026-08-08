# Titanweave K14.C12 — Trusted Radeon IP Bases + First Live Status Reads

C12 is forked from frozen/qualified K14.C11. It closes the address-resolution gap between reviewed AMD register indices and an actual CPU-visible Radeon register address.

## New contract

- Modern Radeon register MMIO uses PCI BAR5, matching upstream `amdgpu_device.c` for Bonaire-and-newer devices.
- Navi 21 receives a source-reviewed base map from AMD's generated `sienna_cichlid_ip_offset.h`: GC instance0 segment0 and SDMA0 instance0 segment0 are both `0x1260` DWORDs.
- Navi 48 remains fail-closed until a trusted AMD IP-discovery record source supplies GC/SDMA bases; Titanweave does not guess GFX12 bases.
- C12 maps only the 4 KiB page containing each target register, supervisor-only/read-only/NX/uncached, then performs one aligned volatile `u32` read.
- The first Navi 21 targets are `GRBM_STATUS`, `GRBM_CHIP_REVISION`, and `SDMA0_STATUS_REG`.
- Radeon MMIO writes, firmware upload, command submission, and bus mastering remain forbidden.

## Physical safety preconditions

A live read requires: C6 persistent exact-requester domain, C9 verified PCI identity, C11 reviewed register set, trusted IP bases, BAR5 presence, and bus mastering still disabled.

## Navi 48

C12 contains the bounded trusted discovery-record selector and self-test, but no unverified base table. Binary acquisition/parsing and/or a reviewed GFX12 base source must satisfy that gate before a 9070-class card is read.
