# K14.C14 Qualification Record

Status: QUALIFIED / FROZEN

Qualification environment: Fedora host running the Titanweave K14.C14 QEMU qualification harness.

Observed result:

- QEMU exit status: 0
- PASS [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
- PASS [C13OK] K14.C13 physical Radeon read-proof engine
- PASS [C14PG] write-promotion policy
- PASS [C14HW] Radeon write-promotion readiness
- PASS [C14RD] K14.C14 write-promotion readiness ready
- PASS [C14OK] K14.C14 controlled write-promotion readiness gate
- PASS [USER] [displayd] K14.C14 controlled Radeon write-promotion readiness gate online
- PASS [USER] [displayd] K14.C14 no physical Radeon in QEMU; write-promotion readiness remains safely deferred
- PASS [RECV] kernel initialization reached stable userspace handoff
- PASS [KERN] K14.C14 alive
- PASS [QUAL] K14.C14 write-promotion-readiness runtime reached intentional post-userspace halt
- PASS [HALT] BSP halted intentionally

Final result:

`Titanweave K14.C14 write-promotion-readiness runtime qualification PASSED.`

Scope note: this qualifies the C14 QEMU/runtime readiness contract. QEMU does not provide a physical Radeon, so physical Radeon write-side promotion remains deferred. MMIO writes, firmware upload, command submission, and Radeon bus mastering remain fenced at this milestone.
