# K11 Compile Fix 1

Fixes exposed by the first real Rust 1.97.1 build on Fedora 44:

- Parenthesizes the `header.length as usize` cast in ACPI MADT length validation so `<` parses as comparison under current Rust.
- Adds a bounds-checked `MemoryBlockDevice::slice(first_sector, sector_count)` implementation required by GPT auto-mount partition probing.

The slice preserves the parent device write-access policy and rejects zero-length, arithmetic-overflow, and out-of-range subdevices.
