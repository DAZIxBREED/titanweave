# Titanweave Build Status

K14.C5 is source-integrated from the frozen, QEMU-qualified K14.C4 baseline. All inherited K1-K14.C4 source gates and the new K14.C5 AMD-Vi page-table gate pass in the packaging environment. Userspace assembly builds successfully here. The packaging environment does not contain Cargo/Rust, so full Rust/QEMU runtime qualification must be performed on Fedora before C5 is frozen. Physical Radeon bus mastering, MMIO writes, firmware upload and command submission remain fenced.

K14.C22: qualified/frozen from the qualified K14.C21 baseline. Fedora/QEMU runtime qualification passed with all C22 gates, userspace handoff, intentional post-userspace halt, and automatic QEMU termination verified.
