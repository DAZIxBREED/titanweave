# Titanweave K14.C21 Linkfix 2 — ELF program-header contract

A Fedora C21 build linked successfully after Linkfix 1, but TitanBoot rejected `WEAVECORE.ELF` with `No usable program headers` before WeaveCore entry.

The failing rust-lld invocation contained the Titanweave linker script and entry argument twice. Reproducing that condition directly with the C21 `PHDRS` linker script yields an x86-64 ET_EXEC image with a normal 56-byte program-header entry size but `e_phnum = 0`. TitanBoot correctly rejects that image.

## Fix

`tools/build.sh` now supplies WeaveCore's compiler/linker arguments through `CARGO_ENCODED_RUSTFLAGS` for the kernel build. Cargo documents that encoded/environment rustflags take precedence over merged `target.*.rustflags`, preventing an older parent/user Cargo configuration from duplicating Titanweave's `-Tkernel/weavecore/linker.ld` setting.

After linking, `tools/check-kernel-elf.py` validates the exact TitanBoot K14 contract before the ESP is staged:

- ELF64 little-endian x86-64 ET_EXEC;
- usable program-header table;
- exactly one PT_LOAD;
- higher-half virtual address;
- matching physical/virtual page offsets;
- PT_LOAD file bounds and sizes valid;
- entry point contained in the PT_LOAD.

This is a build/ELF packaging correction only. It does not change the K14.C21 Radeon MMIO policy or qualification boundary.
