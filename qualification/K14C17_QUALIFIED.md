# Titanweave K14.C17 Qualification Record

K14.C17 — AMD IP-discovery / Navi48 base-resolution foundation

Runtime qualification result: PASSED

Observed QEMU qualification markers:

- QEMU exit status: 0
- [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
- [C16OK] K14.C16 reviewed Radeon MMIO-write gate
- [C17IP] AMD IP-discovery parser
- [C17PG] Navi48 resolution policy
- [C17HW] Navi48 IP discovery
- [C17RD] K14.C17 IP-discovery gate ready
- [C17OK] K14.C17 AMD IP-discovery/Navi48 base-resolution gate
- [USER] displayd K14.C17 AMD IP-discovery/Navi48 base-resolution gate online
- [USER] displayd no physical Radeon in QEMU; live IP-discovery fetch safely deferred
- [RECV] kernel initialization reached stable userspace handoff
- [KERN] K14.C17 alive
- [QUAL] K14.C17 IP-discovery runtime reached intentional post-userspace halt
- [HALT] BSP halted intentionally

Final result:

`Titanweave K14.C17 IP-discovery runtime qualification PASSED.`

## Qualification boundary

This freezes the K14.C17 QEMU/runtime qualification path and the bounded AMD IP-discovery parser/policy framework. It does not claim that a physical Navi48 discovery snapshot was fetched or that a physical Radeon MMIO write occurred. Live physical IP-discovery access remains fail-closed until a later milestone explicitly promotes and validates that path.
