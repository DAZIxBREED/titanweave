# K14.C23 integration delta

K14.C23 is developed from frozen/qualified K14.C22. This branch records the exact milestone delta plus qualification material. `K14C23_INTEGRATION.patch` is an exact source diff from the frozen C22 integrated tree to the qualified C23 integrated tree, excluding generated `build/`, `target/`, and `SOURCE_MANIFEST.sha256` artifacts.

Integrated C23 changes include:

- `kernel/weavecore/src/native_gpu_c23.rs`: post-C22 restoration persistence verification plus two distinct internally-derived one-bit `SCRATCH_REG0` probe/readback/restore cycles.
- `kernel/weavecore/src/main.rs`: C23 initialization and `[C23OK]` milestone marker.
- `kernel/weavecore/src/abi.rs`, `syscalls.rs`, `userspace/include/twabi.inc`: syscall 34 / C23 query ABI wiring.
- `userspace/displayd/displayd.S`: C23 userspace status and explicit QEMU-safe deferred reporting.
- `kernel/weavecore/src/process.rs`: C23 alive/qualification markers.
- `tools/test-k14c23-dual-probe-stability.py`: source-contract validation.
- `tools/check-k14c23-serial-log.sh`: runtime PASS checker.
- `tools/run-k14c23-qemu-dual-probe-stability.sh`: automatic intentional-HALT QEMU qualification runner.
- `tools/validate-source.sh`: C23 validation integration.

Safety remains narrow: C23 opens no new register address and accepts no caller-selected MMIO value. Arbitrary MMIO writes, MM_INDEX fallback, BAR resizing, firmware upload, GPU command submission, and Radeon bus-master enable remain fenced.

Runtime qualification passed on Fedora/QEMU on 2026-08-09. QEMU validates the gated/deferred path only; physical Navi48 dual-probe mutation/restoration remains a separate bare-metal proof.
