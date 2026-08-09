# Titanweave K14.C25 Runtime Qualification

Status: **QUALIFIED / FROZEN**

K14.C25 runtime qualification was confirmed by the project owner before K14.C26 integration. The qualified path preserves the exact checksum-backed GFX12 `SCRATCH_REG0` target, two distinct reversible four-bit pattern/readback/restore cycles, inter-cycle persistence, userspace handoff, intentional post-userspace halt, and the halt-aware QEMU runner.

The exact serial transcript was not embedded into the C26 packaging environment, so this record intentionally does not invent one.
