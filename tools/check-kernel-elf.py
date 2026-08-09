#!/usr/bin/env python3
"""Fail the build before QEMU if WEAVECORE.ELF violates TitanBoot's ELF contract."""
from pathlib import Path
import struct
import sys

path = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/x86_64-unknown-none/debug/weavecore')
data = path.read_bytes()

def fail(msg: str) -> None:
    raise SystemExit(f'WEAVECORE.ELF validation failed: {msg}')

if len(data) < 64 or data[:4] != b'\x7fELF':
    fail('ELF64 header missing')
if data[4:7] != bytes((2,1,1)):
    fail('expected little-endian ELF64 v1')
(e_type, e_machine) = struct.unpack_from('<HH', data, 16)
if e_type != 2 or e_machine != 62:
    fail(f'expected x86-64 ET_EXEC, got type={e_type} machine={e_machine}')
entry, phoff = struct.unpack_from('<QQ', data, 24)
phentsize, phnum = struct.unpack_from('<HH', data, 54)
if phentsize < 56 or phnum == 0:
    fail(f'no usable program headers (phentsize={phentsize}, phnum={phnum}); check for duplicated -T linker-script flags')
if phoff + phentsize * phnum > len(data):
    fail('program-header table lies outside file')
loads = []
for i in range(phnum):
    off = phoff + i * phentsize
    p_type, p_flags = struct.unpack_from('<II', data, off)
    if p_type != 1:
        continue
    p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_align = struct.unpack_from('<QQQQQQ', data, off + 8)
    if p_memsz == 0 or p_filesz > p_memsz:
        fail(f'PT_LOAD[{i}] has invalid sizes')
    if p_offset + p_filesz > len(data):
        fail(f'PT_LOAD[{i}] data lies outside file')
    loads.append((p_vaddr,p_paddr,p_filesz,p_memsz,p_flags,p_align))
if len(loads) != 1:
    fail(f'TitanBoot K14 requires exactly one PT_LOAD, found {len(loads)}')
vaddr,paddr,filesz,memsz,flags,align = loads[0]
if vaddr < 0xffff_ffff_8000_0000:
    fail(f'PT_LOAD is not in higher half: {vaddr:#x}')
if (vaddr & 0xfff) != (paddr & 0xfff):
    fail('PT_LOAD virtual/physical page offsets differ')
if not (vaddr <= entry < vaddr + memsz):
    fail(f'entry {entry:#x} is outside PT_LOAD')
print(f'WEAVECORE.ELF validated: entry={entry:#x} PT_LOAD vaddr={vaddr:#x} paddr={paddr:#x} filesz={filesz:#x} memsz={memsz:#x}')
