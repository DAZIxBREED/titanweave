# Titanweave K14.C21 — Reviewed GFX12 target rebind and identity-write gate

K14.C21 closes the exact-address gap that remained between frozen K14.C16 and the checksum-qualified live discovery chain in K14.C19/K14.C20.

## Source-backed target

For GFX12, AMD's generated `gc_12_0_0_offset.h` defines `regSCRATCH_REG0 = 0x2040` and `regSCRATCH_REG0_BASE_IDX = 1`. AMDGPU's `SOC15_REG_OFFSET` rule computes a register dword address as `reg_offset[IP][instance][BASE_IDX] + reg`. Therefore C20's slot-0 GC base cannot be used for this target. C21 re-opens only the already checksum-qualified C19 snapshot and extracts instance-0 GC base-address slot 1.

C21 then computes:

`target_dwords = GC.base_address[1] + 0x2040`

`target_bytes = target_dwords * 4`

No static Navi48 base is invented.

## Post-discovery gate

C14/C15/C16 are frozen pre-discovery milestones and are not rewritten. For Navi48, their earlier runtime state can remain deferred because the trusted-base and physical-read facts did not yet exist. C21 derives a new post-discovery gate from the same safety invariants plus the later proofs:

- exact verified Navi48 PCI profile
- persistent translated DMA domain
- C16 semantic target reviewed as GFX12 `SCRATCH_REG0`
- C19 physical snapshot acquired and checksum-verified with nonzero fingerprint
- C20 exact GFX12 GC/SDMA proof ready
- exact GC `BASE_IDX=1` resolved from that verified snapshot
- BAR5 register aperture present
- PCI memory decoding enabled
- PCI bus mastering clear immediately before the transaction

## Only promoted write

When every physical gate is true, C21 permits one reviewed identity transaction:

1. read current `SCRATCH_REG0` u32;
2. reject an all-ones response;
3. write the exact same u32 back;
4. poll at most 32 reads for exact equality;
5. if equality fails, attempt one restore write of the original value and fail qualification;
6. re-read PCI Command and require memory decoding still on and bus mastering still off.

The successful path performs one MMIO store. The maximum bounded count is two only to allow one restore attempt.

## Still forbidden

Arbitrary Radeon MMIO writes, MM_INDEX/MM_DATA fallback, BAR resizing, firmware upload, GPU command submission, and Radeon bus-master enable remain disabled. QEMU contains no physical Radeon and must take the explicit deferred path.
