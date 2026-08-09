#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(rel): return (root/rel).read_text()

auto=text('kernel/weavecore/src/automount.rs')
ntfs=text('kernel/weavecore/src/ntfs.rs')
assert 'ReadOnlyFoundation' not in auto, 'stale nonexistent NTFS enum variant returned'
for state in ['CleanReadOnly','DirtyReadOnly','HibernatedReadOnly']:
    assert state in ntfs and state in auto, f'NTFS safety state not handled: {state}'

main=text('kernel/weavecore/src/main.rs')
process=text('kernel/weavecore/src/process.rs')
assert 'recovery::mark_boot_failed();' in main.split('#[panic_handler]',1)[1], 'panic does not record failed boot'
assert 'recovery::mark_boot_stable();' not in main.split('#[panic_handler]',1)[1], 'panic incorrectly marks boot stable'
assert 'crate::recovery::mark_boot_stable();' in process, 'successful userspace handoff never marks boot stable'

nvme=text('kernel/weavecore/src/nvme_full.rs')
for token in ['**cid == c.cid','NVMe completion CID is not inflight','BlockOperation::Flush','blocks != 0 || prp1 != 0 || prp2 != 0','validate_data_prps']:
    assert token in nvme, f'NVMe semantic closure missing: {token}'
assert 'find(|x|**x)' not in nvme, 'NVMe still retires arbitrary first inflight request'

xhci=text('kernel/weavecore/src/xhci.rs')
assert 'pub transfer: TrbRing' in xhci, 'xHCI has no transfer ring'
control=xhci.split('pub fn submit_control',1)[1]
assert 'self.transfer.push' in control, 'USB control TD not submitted to transfer ring'
assert 'self.command.push' not in control, 'USB control TD incorrectly submitted to command ring'

irq=text('kernel/weavecore/src/interrupt_router.rs')
forge=text('kernel/weavecore/src/forgebus.rs')
for token in ['DeviceInterruptHandler','register_handler','handler: Option<DeviceInterruptHandler>']:
    assert token in irq, f'interrupt handler registration missing: {token}'
assert 'handler(vector,device)?;' in forge, 'ForgeBus still accounts interrupts without invoking backend handler'

# Vector 0x80 is the userspace syscall ABI and sits inside the otherwise
# device-owned 0x50..0xdf range. It must remain DPL=3 in the IDT and must never
# be leased to MSI/MSI-X routing. A previous ordering bug overwrote the syscall
# gate and caused #GP(0x402) at every userspace `int 0x80`.
idt=text('kernel/weavecore/src/arch/x86_64/idt.rs')
assert 'if vector == abi::SYSCALL_VECTOR as usize' in idt, 'device IDT population can overwrite syscall vector'
assert idt.rfind('IdtEntry::user_interrupt_gate') > idt.rfind('for index in 0..DEVICE_VECTOR_COUNT'), 'syscall gate is not finalized after device gates'
assert 'SYSCALL_VECTOR' in irq, 'interrupt router does not know the reserved syscall vector'
assert 'raw_vector >= SYSCALL_VECTOR' in irq, 'interrupt router can allocate syscall vector 0x80'


runtime=text('kernel/weavecore/src/kernel_runtime.rs')
backends=text('kernel/weavecore/src/k11_backends.rs')
assert 'static RUNTIME: SpinLock<KernelRuntime>' in runtime, 'kernel runtime still uses unsynchronized UnsafeCell'
assert 'UnsafeCell<KernelRuntime>' not in runtime, 'kernel runtime retains unsafe shared mutable global'
assert 'SpinLock<BackendRuntime>' in backends, 'K11 backend runtime is not SMP-safe'
assert 'UnsafeCell<BackendRuntime>' not in backends, 'K11 backend runtime retains unsafe shared mutable global'
assert 'SpinLock<ForgeBusRuntime>' in forge, 'ForgeBus runtime is not SMP-safe'

msi=text('kernel/weavecore/src/msi.rs')
for token in ['enable_msi(','enable_msix_entry(','MSI Enable','MSI-X Enable']:
    assert token in msi, f'MSI programming path missing: {token}'

print('Titanweave K11 semantic regression checks passed.')
