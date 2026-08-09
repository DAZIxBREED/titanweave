#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
s=(root/'kernel/weavecore/linker.ld').read_text()
for x in ['.got ALIGN(0x1000)','*(.got .got.*)','*(.got.plt)','*(.debug*)','*(.zdebug*)','*(.gdb_index)']:
    assert x in s, x
assert s.index('.got ALIGN(0x1000)') < s.index('.data ALIGN(0x1000)')
print('Titanweave K14.C21 linker-layout regression checks passed.')
