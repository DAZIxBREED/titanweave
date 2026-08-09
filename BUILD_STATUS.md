# Titanweave Build Status

K14.C5 is source-integrated from the frozen, QEMU-qualified K14.C4 baseline. All inherited K1-K14.C4 source gates and the new K14.C5 AMD-Vi page-table gate pass in the packaging environment. Userspace assembly builds successfully here. The packaging environment does not contain Cargo/Rust, so full Rust/QEMU runtime qualification must be performed on Fedora before C5 is frozen. Physical Radeon bus mastering, MMIO writes, firmware upload and command submission remain fenced.

K14.C22: qualified/frozen. Bounded reversible GFX12 SCRATCH_REG0 mutation and exact restoration passed Fedora/QEMU qualification; physical Radeon execution remains a separate bare-metal proof.

K14.C23: qualified/frozen from the qualified K14.C22 baseline. Fedora/QEMU runtime qualification passed with C22-restoration persistence, both distinct bounded one-bit probe/restore cycles, userspace handoff, intentional post-userspace halt, and automatic QEMU termination verified. Physical Radeon execution remains a separate bare-metal proof; no additional Radeon register authority is opened.

K14.C24: qualified/frozen from the qualified K14.C23 baseline. Fedora/QEMU runtime qualification passed the deterministic reversible four-bit SCRATCH_REG0 pattern/readback/restore gate, userspace handoff, intentional post-userspace halt, and automatic QEMU termination. Physical Radeon execution remains a separate bare-metal proof; no additional Radeon register authority is opened.
