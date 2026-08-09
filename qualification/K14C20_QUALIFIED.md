# Titanweave K14.C20 Qualification Record

Status: QUALIFIED / FROZEN

Milestone: K14.C20 — AMD exact live IP-base resolver

Authoritative runtime qualification supplied by the project owner:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C19OK] K14.C19 physical AMD discovery snapshot gate:
PASS  [C20IP] AMD live IP-record resolver:
PASS  [C20PG] exact-base policy:
PASS  [C20HW] AMD exact IP bases:
PASS  [C20RD] K14.C20 exact-base gate ready:
PASS  [C20OK] K14.C20 AMD exact live IP-base gate:
PASS  [USER] [displayd] K14.C20 AMD exact live IP-base resolver online
PASS  [USER] [displayd] K14.C20 no physical Radeon in QEMU; verified-snapshot IP-base resolution remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [KERN] K14.C20 alive:
PASS  [QUAL] K14.C20 exact-IP-base runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C20 exact-IP-base runtime qualification PASSED.
```

Qualification meaning:
- QEMU runtime/policy path passed.
- The AMD IP-record resolver and exact-base gate reached stable userspace handoff and the intentional qualification halt.
- No physical Radeon was present in QEMU, so a live bare-metal Navi48 snapshot/base-resolution proof remains separate.
- Radeon MMIO writes, firmware upload, command submission, and bus-master enable remain outside this qualification.
