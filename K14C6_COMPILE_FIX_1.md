# Titanweave K14.C6 Compile Fix 1

## Symptom
Rust E0609 in `kernel/weavecore/src/main.rs`: C6 attempted to read `boot_info.kernel_cr3`, but `BootInfo` has no top-level `kernel_cr3` field.

## Fix
C6 now receives the already-established bootstrap page-table root used by the earlier native GPU path:

```rust
native_gpu_c6::initialize(&mut allocator, boot_info.bootstrap.page_table_root)
```

The K14.C6 source regression test was updated to require the corrected call. A stale unused `gpu_topology` import in `native_gpu_c3.rs` was also removed.

## Qualification status
Static K1-K14.C6 integrated source validation passes after this fix. Runtime QEMU qualification is still required on the tester machine.
