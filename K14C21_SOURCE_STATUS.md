# Titanweave K14.C21 Source Status

Status: IMPLEMENTED / SOURCE-VALIDATED / RUNTIME QUALIFICATION PENDING

K14.C21 reconnects the checksum-qualified C19/C20 AMD discovery chain to the reviewed GFX12 `SCRATCH_REG0` identity-write transaction.

Key safety correction: GFX12 `SCRATCH_REG0` is generated with `BASE_IDX=1`, so C21 resolves GC base-address slot 1 from the already verified discovery snapshot and cross-checks GC slot 0/version against frozen C20. It does not substitute C20's slot-0 base for this register.

The Fedora/QEMU qualification harness defaults to GTK graphics while collecting the serial qualification log.

Still forbidden: arbitrary Radeon MMIO writes, MM_INDEX fallback, BAR resizing, firmware upload, GPU command submission, Radeon bus-master enable, guessed register offsets, and guessed IP bases.

A QEMU PASS qualifies the runtime/gating/safe-defer path only. A physical Radeon MMIO identity-write remains a separate bare-metal proof.
