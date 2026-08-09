# K12 Compile Fix 1

First real Rust compilation of K12 exposed two source-level defects that static validation did not catch.

1. `PresentRequest` declared a 48-byte ForgeGraphics ABI contract but only contained 40 bytes of fields. The v1 contract remains 48 bytes by adding an explicit reserved 64-bit tail field rather than silently shrinking the ABI.
2. `InputRouter` attempted to call `self.next_sequence()` while `self` was already mutably borrowed by `self.push(...)`. Sequence values and pointer coordinates are now captured into locals before queue insertion.

This is a compile-correctness fix; K12 QEMU runtime/display qualification remains required.
