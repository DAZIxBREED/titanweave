# Titanweave K14.C24 Source Status

Status: **QUALIFIED / FROZEN**

K14.C24 is frozen from qualified K14.C23. It keeps the exact checksum-backed GFX12 `SCRATCH_REG0` target and advances only to one deterministic reversible four-bit pattern with exact readback and mandatory original-value restoration. Fedora/QEMU runtime qualification passed through `[C24OK]`, userspace handoff, `[QUAL]`, and intentional `[HALT]` with automatic QEMU termination.

No additional Radeon register authority or destructive capability is enabled. QEMU does not prove the physical bare-metal store; physical Navi48 execution remains separately gated.
