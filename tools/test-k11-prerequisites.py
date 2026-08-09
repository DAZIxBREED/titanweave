#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
checks={
 'kernel/weavecore/src/paging.rs':['pub fn destroy','deallocate_frame','Kernel PML4 mappings'],
 'kernel/weavecore/src/block_queue.rs':['BlockRequestQueue','deadline_tick','cancel_owner','scatter/gather'],
 'kernel/weavecore/src/object_lifecycle.rs':['ObjectLifecycle','generation','reference underflow'],
 'kernel/weavecore/src/interrupt_router.rs':['record_dispatch','interrupt arrived while masked','migrate'],
 'kernel/weavecore/src/driver_watchdog.rs':['DriverWatchdog','WatchdogAction::Restart','WatchdogAction::Quarantine'],
 'docs/architecture/K11-PREREQUISITES.md':['K11.0','reclaimable user address spaces'],
}
for path,tokens in checks.items():
 data=text(path)
 for token in tokens:
  if token not in data: raise SystemExit(f'{path}: missing {token}')
print('Titanweave K11.0 prerequisite closure checks passed.')
