# K14.C21 integration delta

K14.C21 is developed from frozen K14.C20. The complete integrated source is distributed as the milestone ZIP; this GitHub branch intentionally records the milestone delta and qualification material rather than claiming to mirror every inherited file.

Integrated C21 changes in the complete source package include:

- `kernel/weavecore/src/native_gpu_c21.rs`: reviewed GFX12 `SCRATCH_REG0` target resolver and bounded same-value MMIO identity-write executor.
- `kernel/weavecore/src/main.rs`: C21 initialization and `[C21OK]` milestone marker.
- `kernel/weavecore/src/abi.rs`: syscall 32 (`SYS_NATIVE_GPU_C21_QUERY`).
- `kernel/weavecore/src/syscalls.rs`: C21 query dispatch.
- `userspace/include/twabi.inc`: C21 syscall number.
- `userspace/displayd/displayd.S`: C21 userspace status and QEMU-safe defer message.
- `kernel/weavecore/src/process.rs`: C21 alive/qualification halt markers.
- `tools/run-k14c21-qemu-reviewed-mmio-rebind.sh`: GTK-default QEMU qualification runner.
- `tools/check-k14c21-serial-log.sh`: runtime PASS checker.
- `tools/test-k14c21-reviewed-mmio-rebind.py`: source-contract validation.
- `tools/validate-source.sh`, `K14_STATUS.md`, and `SOURCE_MANIFEST.sha256`: C21 integration updates.

Safety boundary: the only newly promotable physical operation is a reviewed same-value write/readback of GFX12 `SCRATCH_REG0` after all post-discovery gates pass. Arbitrary MMIO writes, MM_INDEX fallback, BAR resizing, firmware upload, GPU submission, and Radeon bus-master enable remain disabled.

Runtime qualification is still required before K14.C21 may be marked qualified/frozen.
