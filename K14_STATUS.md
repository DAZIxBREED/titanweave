# Titanweave K14 Status

- K14.A native GPU prerequisite foundation: qualified and frozen.
- K14.B hardware-translated DMA / Intel VT-d qualification: qualified and frozen.
- K14.C1 native GPU ownership foundation: qualified and frozen.
- K14.C2 persistent-domain + AMD bring-up contract: qualified and frozen.
- K14.C3 Radeon bare-metal staging: qualified and frozen.
- K14.C4 exact Radeon requester / AMD-Vi qualification gate: source-integrated; runtime qualification pending.

K14.C4 remains fail-closed. It does not claim a production AMD-Vi page-table engine, Radeon firmware upload, command submission, MMIO writes, or Radeon bus mastering.


## K14.C5

AMD-Vi hardware page-table engine foundation added: exact requester DTE image, pinned device/page tables, command buffer, event log and fault-path state. Physical register programming and Radeon bus mastering remain fail-closed pending bare-metal AMD-Vi qualification.


## K14.C6
Live AMD-Vi hardware-programming boundary added; QEMU must remain fail-closed and bare-metal activation is separately gated.

## K14.C7 — Radeon MMIO / firmware discovery staging

Source-integrated after qualified K14.C6. Adds a supervisor-only read-only Radeon MMIO mapper, exact-domain-gated BAR promotion, PCI identity capture, VBIOS/firmware discovery planning, and GMC/GTT readiness staging. Radeon register reads/writes, firmware upload, command submission, and bus mastering remain fenced pending runtime/bare-metal qualification.

## K14.C8
Radeon ASIC/IP identification and side-effect-free safe-register-read gate. QEMU qualification requires no physical Radeon and preserves all destructive capabilities fenced.

## K14.C9 — Verified Radeon Profiles + Live Safe Identity Reads

C9 adds grounded Navi21 (`1002:73bf`) and Navi48 (`1002:7550`) bring-up profiles and side-effect-free live PCI identity/status reads. Radeon MMIO reads/writes, firmware upload, submission, and bus mastering remain fenced pending exact per-IP MMIO whitelist verification.


## K14.C10
Per-IP Radeon MMIO whitelist engine and guarded live-read activation gate added; physical offsets remain fail-closed until exact IP-specific review.


## K14.C11
Reviewed Radeon register definitions and IP-base resolver gate. C10 is frozen; C11 carries AMD-upstream-derived GC/SDMA register indices for Navi 21 and Navi 48, but physical dereference remains fail-closed until trusted per-IP base addresses are resolved.


## K14.C12
Trusted Radeon IP-base sources and first bounded live status-read path. QEMU qualification pending. Write-side Radeon paths remain fenced.

## K14.C13 — Physical Radeon read-proof qualification
Status: source-ready; QEMU/runtime qualification pending. C12 remains frozen. C13 adds immutable read evidence, sanity checks, bus-master recheck, and a fail-closed Navi48 discovery-pending state.

## K14.C14
Controlled Radeon write-promotion readiness gate implemented. Actual writes, firmware upload, command submission, and Radeon bus mastering remain disabled pending later qualification.


## K14.C15
First controlled write transaction: width-correct 16-bit Radeon PCI Command identity write with readback, bounded rollback, bus-master-off verification, and transaction fingerprinting. Radeon MMIO writes, firmware upload, command submission, and bus-master enable remain fenced.

## K14.C16
Reviewed Radeon MMIO identity-write target gate. Exact target/base remain fail-closed where source-backed data is unavailable.

## K14.C17
Bounded AMD IP Discovery binary parser and Navi48 exact-base resolution foundation.

## K14.C18
AMD discovery binary/IP-table checksum verifier plus discovery-TMR acquisition contract. Physical snapshot fetch remained fenced.

## K14.C19
First bounded physical AMD discovery snapshot acquisition path. Reads source-backed TMR location registers from BAR5, inspects BAR0's current Resizable-BAR size through read-only ECAM, and reads the TMR only when the complete range is already visible. MM_INDEX fallback, Radeon MMIO writes, firmware upload, command submission, and bus-master enable remain fenced.


## K14.C22 — Reversible SCRATCH_REG0 mutation
Status: qualified/frozen. Fedora/QEMU passed the source/runtime/userspace safe-defer path with automatic intentional-HALT termination. Physical Navi48 execution remains separately qualified on bare metal.

## K14.C23 — Post-restore persistence + dual-probe stability
Status: qualified/frozen. Fedora/QEMU passed the C23 source/runtime/userspace safe-defer path with automatic intentional-HALT termination. Physical Navi48 dual-probe execution remains separately qualified on bare metal.


## K14.C24 — Reversible four-bit SCRATCH_REG0 pattern
Status: qualified/frozen. Fedora/QEMU passed the C24 source/runtime/userspace safe-defer path with automatic intentional-HALT termination. C24 keeps the exact C21-C23 target, requires C23 restoration persistence, applies one deterministic internally-derived four-bit pattern, verifies exact readback, and mandatorily restores the original value. No new register or destructive authority is enabled.

## K14.C25 — Dual reversible four-bit SCRATCH_REG0 pattern stability
Status: qualified/frozen. C25 keeps the exact C21-C24 target, requires C24 restoration persistence, performs two distinct internally-derived four-bit pattern/readback/restore cycles with inter-cycle persistence, and keeps all wider Radeon write capabilities fenced.

## K14.C26 — Safe reviewed MMIO foundation / SCRATCH_REG1 read proof
Status: **qualified/frozen**. Fedora/QEMU passed the source-reviewed GFX12 `SCRATCH_REG1` target at `0x2041` / BASE_IDX 1, same verified GC base-slot/adjacency contract, two-entry reviewed REG0/REG1 MMIO allowlist, bounded read-only REG1 proof, userspace handoff and intentional `[HALT]`. C26 performs zero MMIO writes. The C26 artifact preserves its historical completion marker, but the project owner's locked roadmap supersedes that planning decision: K14 continues through C32 and K15 is ForgeAudio.

## K14.C27 — Complete Radeon driver core
Status: **qualified/frozen**. Fedora/QEMU passed C27 operational driver lifecycle/error/reset coordination, exact ForgeBus ownership, live resource/topology capture, permanent identity-based reviewed-MMIO reads with generic-write rejection, the real interrupt-router route/handler self-test, userspace status ABI, stable userspace handoff, `[K14FOUND]`, `[QUAL]`, and intentional `[HALT]`. QEMU contains no physical Radeon, so physical ownership/MMIO remains safely deferred. No TODO/stub/placeholder subsystem is allowed. C27 adds zero new register authority and leaves firmware upload, DMA/bus mastering, command submission and physical interrupt enable fenced.

## K14.C28 — Memory + firmware + recovery
Status: **qualified/frozen**. Fedora/QEMU passed the operational GTT allocation/map/reclaim, VRAM reservation model, GPU-VA reservation model, AMD firmware common-header/CRC32/SHA staging path, executable watchdog/resource-safe software recovery, userspace handoff, `[QUAL]`, and intentional `[HALT]`. QEMU has no physical Radeon, so firmware silicon upload and physical ASIC reset are not claimed. C29 retains exclusive authority for GPU page tables, bus mastering/DMA, rings/queues/fences, command submission, and physical GPU interrupt programming.

## Locked remainder of K14
C29 = rings + queues + fences + DMA; C30 = complete basic display engine; C31 = graphics + compute execution; C32 = production/stability + final K14. After C32, K15 begins ForgeAudio.
