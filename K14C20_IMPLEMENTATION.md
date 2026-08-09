# Titanweave K14.C20 — Exact AMD IP Base Resolution

K14.C20 consumes only the checksum-qualified snapshot exported by frozen K14.C19. It walks AMD's packed IP-discovery die/IP records with bounded offsets and extracts exact instance-0 GC and SDMA base-address records.

Source facts imported for this milestone:

- AMD `GC_HWID = 11`.
- AMD `SDMA0_HWID = 42`, `SDMA1_HWID = 43`, `SDMA2_HWID = 68`, `SDMA3_HWID = 69`.
- `ip_v4` records contain an 8-byte fixed prefix plus a variable base-address list.
- IP-discovery v4 may encode base addresses as 64-bit values.
- Upstream AMDGPU truncates v4 64-bit base entries to their low 32 bits and masks with `0x3fffffff` because register bases are dword-addressed.

C20 requires the C19 snapshot to already have passed both AMD binary and IP-table checksums. It performs no hardware access of its own.

## Promotion boundary

For a Navi48 profile, C20 exposes `c16_promotion_input_ready` only when the verified live snapshot contains a nonzero instance-0 GC base, a nonzero instance-0 SDMA0 base, and the GC record reports major version 12 (GFX12). This is input for a later milestone; C20 does not mutate or retroactively change frozen C16.

## Still fenced

- Radeon MMIO writes
- firmware upload
- GPU command submission
- Radeon bus-master enable
- guessed register/IP bases

QEMU has no physical Radeon and must qualify the explicit deferred path.
