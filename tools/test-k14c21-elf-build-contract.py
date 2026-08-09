#!/usr/bin/env python3
from pathlib import Path
import subprocess, tempfile, textwrap
root=Path(__file__).resolve().parents[1]
b=(root/'tools/build.sh').read_text()
for x in [
    'KERNEL_RUSTFLAGS=$\'-C\\x1flink-arg=-Tkernel/weavecore/linker.ld',
    'CARGO_ENCODED_RUSTFLAGS="$KERNEL_RUSTFLAGS"',
    'python3 "$ROOT/tools/check-kernel-elf.py" "$KERNEL_ELF"',
]: assert x in b,x
c=(root/'tools/check-kernel-elf.py').read_text()
for x in ['phnum == 0','duplicated -T linker-script flags','requires exactly one PT_LOAD','entry {entry:#x} is outside PT_LOAD']:
    assert x in c,x
print('Titanweave K14.C21 sealed-rustflags/ELF-contract source checks passed.')
