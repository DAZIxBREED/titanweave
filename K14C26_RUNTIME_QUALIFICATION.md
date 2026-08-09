# Titanweave K14.C26 Runtime Qualification

Status: **QUALIFIED / FROZEN**

K14.C26 completed Fedora/QEMU runtime qualification successfully on 2026-08-09. The final K14 completion gate passed the inherited frozen C25 contract, exact reviewed GFX12 `SCRATCH_REG1` resolution, two-entry REG0/REG1 MMIO allowlist, bounded read-only REG1 proof, stable userspace handoff, explicit K14 completion marker, intentional post-userspace halt, and automatic QEMU termination.

QEMU has no physical Radeon, so this qualifies the C26 source/self-test/runtime-gating/userspace/deferred path. It does not claim physical Navi48 `SCRATCH_REG1` MMIO reads occurred on bare metal. C26 performs zero MMIO writes and does not expand firmware upload, GPU command submission, BAR resize, MM_INDEX fallback, caller-supplied MMIO, or Radeon bus-master authority.

Observed qualification ending:

```text
Intentional Titanweave HALT detected; terminating QEMU.
qemu: terminating on signal 15 from pid 194121 (bash)
PASS  [BOOT] WeaveCore K14 entered from WEAVECORE.ELF
PASS  [C25OK] K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate:
PASS  [C26RV] second reviewed GFX12 target:
PASS  [C26AL] final K14 MMIO allowlist:
PASS  [C26PG] K14 completion policy:
PASS  [C26HW] GFX12 SCRATCH_REG1 read proof:
PASS  [C26RD] K14.C26 completion ready:
PASS  [C26OK] K14.C26 final reviewed GFX12 MMIO allowlist/read-only completion gate:
PASS  [USER] [displayd] K14.C26 final GFX12 reviewed MMIO allowlist and SCRATCH_REG1 read-only completion gate online
PASS  [USER] [displayd] K14.C26 no physical Radeon in QEMU; final K14 SCRATCH_REG1 read proof remains safely deferred
PASS  [RECV] kernel initialization reached stable userspace handoff
PASS  [KERN] K14.C26 alive:
PASS  [QUAL] K14.C26 final-k14-mmio-allowlist runtime reached intentional post-userspace halt
PASS  [K14DONE] K14 native Radeon foundation completion gate reached; broader driver bring-up moves to K15
PASS  [HALT] BSP halted intentionally
Titanweave K14.C26 final-k14-mmio-allowlist runtime qualification PASSED.
QEMU stopped after intentional kernel halt (raw exit status: 0)
```

K14.C26 is therefore frozen, and **K14 is complete/frozen**. Broader native Radeon driver bring-up begins in K15.
