# K9 Validation

Run `./tools/validate-source.sh` for source-contract, assembly, userspace ELF, and FAT32-image checks.

Full K9 acceptance additionally requires Rust compilation and QEMU/OVMF boot testing, followed by integration of the upstream 7-Zip codec engine into the restricted Titan Archive Service. Security tests must cover traversal, symlink escapes, malformed headers, encrypted archives, archive bombs, cancellation, removable-media loss, transaction rollback, and power interruption.

## K11 runtime-closure gate

`tools/test-k11-runtime-closure.py` rejects a package that lacks persistent ForgeBus state, called process teardown, two-phase DMA cancellation, device interrupt gates, executed watchdog recovery, or corrected allocator accounting. This is a source-wiring gate, not a substitute for compilation or hardware execution.

## K13.C buffered-presentation gate

`tools/test-k13c-present.py` requires the live K13.B transport to be reused
rather than reclaimed, three presentation buffers, checked partial-damage
transfers, VirtIO fence-echo validation, frame-pacing/fallback policy, and a
DISPLAYD-only graphics-present capability. `tools/check-k13c-serial-log.sh`
then requires those paths to execute in QEMU and reach the intentional K13.C
post-userspace halt.

This packaging environment does not contain Rust/Cargo or QEMU, so full Rust
compilation and K13.C runtime qualification must be performed on the Fedora
maintainer host before K13.C can be frozen.


K14.C11 adds `tools/test-k14c11-reviewed-registers.py`, `tools/check-k14c11-serial-log.sh`, and `tools/run-k14c11-qemu-reviewed-registers.sh`.
