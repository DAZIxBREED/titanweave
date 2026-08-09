#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
checks = {
    'reclaiming frame allocator': ('kernel/weavecore/src/memory.rs', 'deallocate_contiguous'),
    'allocator overlap rejection': ('kernel/weavecore/src/memory.rs', 'overlaps free memory'),
    'queued IPC': ('kernel/weavecore/src/ipc.rs', 'CHANNEL_QUEUE_DEPTH'),
    'IPC endpoint closure': ('kernel/weavecore/src/ipc.rs', 'pub fn close'),
    'handle close': ('kernel/weavecore/src/handles.rs', 'pub fn close'),
    'mutable namespace': ('kernel/weavecore/src/namespace.rs', 'pub fn unregister'),
    'general shared memory': ('kernel/weavecore/src/shared_memory.rs', 'MAX_SHARED_OBJECTS'),
    'block device contract': ('kernel/weavecore/src/block.rs', 'pub trait BlockDevice'),
    'persistent archive service': ('userspace/archive/archive.S', 'service_loop:'),
    'persistent console service': ('userspace/console/console.S', 'service_loop:'),
    'persistent log service': ('userspace/logd/logd.S', 'service_loop:'),
}
for name, (path, token) in checks.items():
    text = (root/path).read_text()
    if token not in text:
        raise SystemExit(f'missing {name}: {path} lacks {token!r}')
for banned in ('run_preemptive_demo', 'bootstrap_demo', 'prove_sector_zero_read'):
    for path in (root/'kernel').rglob('*'):
        if path.is_file() and banned in path.read_text(errors='ignore'):
            raise SystemExit(f'legacy scaffolding symbol {banned!r} remains in {path}')
print('Titanweave K1-K6 foundation replacement checks passed.')
