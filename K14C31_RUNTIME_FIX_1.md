# Titanweave K14.C31 Runtime Fix 1 — Shader Wire-Magic Endianness

Observed Fedora/QEMU failure:

`[FAIL] K14.C31 graphics+compute execution failed: C31 shader magic invalid`

Root cause: the reference shader blobs begin with the literal bytes `TWSH`, while the parser reads those four bytes using `u32::from_le_bytes`. The original numeric constant was written in big-endian display order (`0x54575348`), so the parser compared the little-endian value of `TWSH` (`0x48535754`) against the wrong integer.

Fix: define `TW_SHADER_MAGIC` from the actual wire bytes with `u32::from_le_bytes(*b"TWSH")`. This preserves the shader blob format and corrects only the parser constant. The C31 source test now validates the actual on-wire magic relationship instead of merely checking for shader symbols.

C29 and C30 remain frozen and unchanged in authority. C31 runtime qualification remains pending until the corrected Fedora/QEMU run passes all C31 gates.

## Post-fix qualification

The corrected Fedora/QEMU rerun passed all C31 runtime gates on 2026-08-09. K14.C31 is now **QUALIFIED / FROZEN**; C32 is the next and final locked Radeon milestone.
