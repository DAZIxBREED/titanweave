# K14.C24 integration delta

K14.C24 is developed from frozen/qualified K14.C23. `K14C24_INTEGRATION.patch.gz` records the exact source delta from the frozen C23 integrated tree to the C24 integrated tree, excluding generated `build/`, `target/`, and `SOURCE_MANIFEST.sha256` artifacts.

Integrated C24 changes include:

- `kernel/weavecore/src/native_gpu_c24.rs`: deterministic reversible four-bit GFX12 `SCRATCH_REG0` pattern/readback/restore gate.
- `kernel/weavecore/src/main.rs`: C24 initialization and `[C24OK]` milestone marker.
- `kernel/weavecore/src/abi.rs`, `syscalls.rs`, `userspace/include/twabi.inc`: syscall 35 / C24 query ABI wiring.
- `userspace/displayd/displayd.S`: C24 userspace reporting and QEMU-safe deferred path.
- `kernel/weavecore/src/process.rs`: C24 alive/qualification markers.
- `tools/test-k14c24-multi-bit-pattern.py`: C24 source-contract validation.
- `tools/check-k14c24-serial-log.sh`: runtime PASS checker.
- `tools/run-k14c24-qemu-multi-bit-pattern.sh`: automatic intentional-HALT QEMU runner.
- `tools/validate-source.sh`: C24 validation integration.
- C24 implementation/status/tester documentation and source-validation evidence.

Runtime qualification passed on Fedora/QEMU on 2026-08-09. Physical Navi48 mutation/readback/restoration remains a separate bare-metal proof.
