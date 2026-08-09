# Titanweave K14.C19 Runtime Qualification

Status: QUALIFIED AND FROZEN

Milestone: K14.C19 — physical AMD discovery snapshot gate

Qualification result:

```text
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C18OK] K14.C18 AMD discovery snapshot-verification gate:
PASS  [C19PG] physical discovery-read policy:
PASS  [C19AP] VRAM aperture contract:
PASS  [C19HW] physical AMD discovery snapshot:
PASS  [C19RD] K14.C19 physical snapshot gate ready:
PASS  [C19OK] K14.C19 physical AMD discovery snapshot gate:
PASS  [USER] [displayd] K14.C19 physical AMD discovery snapshot gate online
PASS  [USER] [displayd] K14.C19 no physical Radeon in QEMU; direct BAR0 discovery snapshot read remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [KERN] K14.C19 alive:
PASS  [QUAL] K14.C19 physical-snapshot runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C19 physical-snapshot runtime qualification PASSED.
```

Qualification boundary:

- QEMU confirms the C19 policy, aperture-contract, runtime wiring, userspace handoff, and safe-defer behavior.
- QEMU does not prove a physical Radeon BAR0 discovery payload was read.
- Physical acquisition remains conditional on a real AMD GPU with the full discovery TMR visible through the CPU-visible BAR0 aperture.
- MM-indexed fallback, BAR resizing, Radeon MMIO writes, firmware upload, command submission, and Radeon bus-master enable remain fenced.

This qualification freezes K14.C19 as the basis for K14.C20.
