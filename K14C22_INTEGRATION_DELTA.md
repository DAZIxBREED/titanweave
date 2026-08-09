# K14.C22 integration delta

K14.C22 is developed from frozen/qualified K14.C21. This branch records the exact milestone delta plus qualification material. `K14C22_INTEGRATION.patch.gz` is an exact compressed source diff from the frozen C21 integrated tree to the frozen C22 integrated tree, excluding generated `build/`, `target/`, and `SOURCE_MANIFEST.sha256` artifacts.

Integrated C22 changes include:

- `kernel/weavecore/src/native_gpu_c22.rs`: bounded reversible one-bit GFX12 `SCRATCH_REG0` mutation with mandatory restoration.
- `kernel/weavecore/src/main.rs`: C22 initialization and `[C22OK]` milestone marker.
- `kernel/weavecore/src/abi.rs`, `syscalls.rs`, `userspace/include/twabi.inc`: C22 query ABI wiring.
- `userspace/displayd/displayd.S`: C22 userspace status and QEMU-safe deferred reporting.
- `kernel/weavecore/src/process.rs`: C22 alive/qualification markers.
- `tools/test-k14c22-reversible-scratch-mutation.py`: source-contract validation.
- `tools/check-k14c22-serial-log.sh`: runtime PASS checker.
- `tools/run-k14c22-qemu-reversible-scratch-mutation.sh`: auto-HALT QEMU qualification runner.
- `tools/validate-source.sh`: C22 validation integration.
- C21 auto-HALT runner correction, now frozen into the inherited harness baseline.

Runtime qualification passed on Fedora/QEMU on 2026-08-09. QEMU validates the gated/deferred path only; physical Navi48 mutation/restoration remains a separate bare-metal proof.
