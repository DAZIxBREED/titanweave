#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
required={
'kernel/weavecore/src/acpi.rs':['pub struct AcpiCatalog','pub fn find(&self,signature:[u8;4]'],
'kernel/weavecore/src/iommu_core.rs':['pub trait TranslationBackend','pub struct FenceToken','block_requester'],
'kernel/weavecore/src/amd_vi.rs':['IVRS','IVHD','impl TranslationBackend for AmdVi'],
'kernel/weavecore/src/intel_vtd.rs':['DMAR','DRHD','impl TranslationBackend for IntelVtd'],
'kernel/weavecore/src/msi.rs':['MsiX','message(cpu_apic_id'],
'kernel/weavecore/src/xhci.rs':['pub struct TrbRing','submit_control'],
'kernel/weavecore/src/usb_hid_full.rs':['decode_mouse','decode_keyboard'],
'kernel/weavecore/src/nvme_full.rs':['submit_io','reset(&mut self)'],
'kernel/weavecore/src/pcie_hotplug.rs':['SurpriseRemoved','stale hot-plug generation'],
'kernel/weavecore/src/k11_stress.rs':['ring.push','bind_device'],
'kernel/weavecore/src/main.rs':['k11_backends::initialize(&catalog)'],
}
for f,tokens in required.items():
 s=(root/f).read_text()
 for token in tokens:
  assert token in s, f'{f}: missing {token}'
print('Titanweave K11.1-K11.8 backend source checks passed.')
