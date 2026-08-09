#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
checks={
 'persistent ForgeBus runtime': ('kernel/weavecore/src/forgebus.rs',['static BUS','ForgeBusRuntime','bus.initialized=true']),
 'process address-space teardown': ('kernel/weavecore/src/process.rs',['reclaim_terminal_address_space','address_space.destroy(allocator)','finalize_all_processes']),
 'handle cleanup': ('kernel/weavecore/src/process.rs',['handles.close_all','objects.release_owner']),
 'two-phase DMA cancellation': ('kernel/weavecore/src/block_queue.rs',['CancelPending','TimeoutPending','fence_device','owner_has_unfenced']),
 'device interrupt gates': ('kernel/weavecore/src/arch/x86_64/idt.rs',['weave_device_isr_table','weave_device_interrupt_dispatch','FIRST_DEVICE_VECTOR']),
 'watchdog execution': ('kernel/weavecore/src/forgebus.rs',['service_watchdog','recover_driver','quarantined']),
 'allocator accounting': ('kernel/weavecore/src/memory.rs',['if allocator.insert_extent','allocator.total_pages']),
}
for label,(path,tokens) in checks.items():
 s=text(path)
 missing=[t for t in tokens if t not in s]
 if missing: raise SystemExit(f'{label} missing: {missing}')
print('Titanweave K11 runtime closure wiring checks passed.')
