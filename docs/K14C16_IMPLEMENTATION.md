# K14.C16 — Reviewed Radeon MMIO Write Gate

K14.C16 introduces the first Radeon MMIO store executor but keeps live execution fail-closed until the exact source-reviewed target and trusted IP base are both resolved.

The initial semantic target is GFX12 GC `SCRATCH_REG0`. Linux AMDGPU's GFX12 ring-test code deliberately writes a magic value to `SCRATCH_REG0` and then verifies a subsequent write, making it an appropriate class of test register. Titanweave does **not** copy or guess a generated numeric register index, and C12 still requires trusted IP-discovery data for Navi48. Therefore the C16 live write remains deferred until both facts are imported and independently validated.

When promoted, the executor is constrained to an identity transaction: read the existing 32-bit value, write that exact value, perform at most 32 bounded readback polls, require exact equality, and recheck that PCI bus mastering remains disabled. A single restore write of the original value is the only rollback path. Any need for rollback fails qualification.

Firmware upload, GPU command submission, and Radeon bus-master enable remain disabled.
