# K14.C12 Qualification Record

Status: **QUALIFIED**

Milestone: K14.C12 — trusted Radeon IP-base / live status-read gate

Qualification environment: Fedora host + QEMU

Result:

```text
QEMU exit status: 0
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C11OK] K14.C11 reviewed register/IP-base gate:
PASS  [C12BS] trusted Radeon IP-base source:
PASS  [C12RM] Radeon register aperture:
PASS  [C12LR] live-read sequence:
PASS  [C12HW] physical Radeon status reads:
PASS  [C12RD] K14.C12 trusted IP-base/live-read path ready:
PASS  [C12OK] K14.C12 trusted IP-base/live-read engine:
PASS  [USER] [displayd] K14.C12 trusted Radeon IP-base and live status-read gate online
PASS  [USER] [displayd] K14.C12 no physical Radeon in QEMU; trusted-base/live-read path remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [KERN] K14.C12 alive:
PASS  [QUAL] K14.C12 trusted-base/live-read runtime reached intentional post-userspace halt
PASS  [HALT] BSP halted intentionally
Titanweave K14.C12 trusted-base/live-read runtime qualification PASSED.
```

K14.C12 is frozen. Later milestones must preserve these regression gates.
