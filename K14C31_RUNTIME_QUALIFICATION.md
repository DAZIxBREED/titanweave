# Titanweave K14.C31 Runtime Qualification

Status: **QUALIFIED / FROZEN**

Fedora/QEMU runtime qualification passed on 2026-08-09 after runtime fix 1 corrected the C31 `TWSH` shader wire-magic endianness.

Passed gates: `[BOOT]`, `[C30OK]`, `[C31SH]`, `[C31CQ]`, `[C31CP]`, `[C31GQ]`, `[C31GX]`, `[C31HC]`, `[C31SC]`, `[C31PG]`, `[C31RD]`, `[C31OK]`, both C31 `displayd` messages, `[RECV]`, `[K14FOUND]`, `[KERN]`, `[QUAL]`, and intentional `[HALT]`. QEMU stopped after the intentional kernel halt with raw exit status 0.

K14.C31 is qualified/frozen. Physical Radeon CP/GFX queues and native AMD ISA execution remain separately gated and are not inferred from QEMU.
