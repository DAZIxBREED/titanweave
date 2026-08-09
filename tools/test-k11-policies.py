#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def text(p): return (root/p).read_text()
checks={
'kernel/weavecore/src/device.rs':['MAX_DEVICES','DeviceState','generation','remove'],
'kernel/weavecore/src/driver.rs':['kernel driver requires system/platform signer','best_match','report_crash','Quarantined'],
'kernel/weavecore/src/dma.rs':['DmaDomain','DMA mask exceeded','unmap','deallocate_contiguous'],
'kernel/weavecore/src/iommu.rs':['IommuBackend','Strict','external DMA device requires translated IOMMU domain'],
'kernel/weavecore/src/interrupt_router.rs':['FIRST_DEVICE_VECTOR','allocate','release','owner'],
'kernel/weavecore/src/hotplug.rs':['HotplugJournal','generation','Removed'],
'kernel/weavecore/src/nvme.rs':['NVME_PROGIF','validate_lba','NVMe LBA overflow'],
'kernel/weavecore/src/usb_hid.rs':['BootKeyboardState','report must be 8 bytes','pressed:false'],
'kernel/weavecore/src/forgebus.rs':['ForgeBus','titan-nvme','titan-usb-hid','pci::enumerate'],
}
for p,tokens in checks.items():
    s=text(p)
    for token in tokens: assert token in s,(p,token)
boot = text('libraries/boot-protocol/src/lib.rs')
import re
match = re.search(r'BOOT_PROTOCOL_VERSION: u32 = (\d+)', boot)
assert match and int(match.group(1)) >= 11
assert 'TITANV' in boot
assert 'DRIVERD.ELF' in text('kernel/weavecore/src/service.rs')
assert (root/'userspace/driverd/driverd.S').is_file()
print('Titanweave K11 ForgeBus policy tests passed.')
