//! K13 PCI graphics topology discovery.
//!
//! Discovery is deliberately side-effect free. Merely finding a GPU must not
//! enable bus mastering, map BARs, or grant DMA. ForgeBus/IOMMU authorization
//! happens later when a backend is ready to take ownership of the adapter.

use crate::pci::{self, PciFunction};

pub const PCI_CLASS_DISPLAY: u8 = 0x03;
pub const VENDOR_AMD: u16 = 0x1002;
pub const VENDOR_INTEL: u16 = 0x8086;
pub const VENDOR_NVIDIA: u16 = 0x10de;
pub const VENDOR_VIRTIO: u16 = 0x1af4;
pub const VIRTIO_GPU_MODERN_DEVICE: u16 = 0x1050;

#[derive(Clone, Copy, Debug, Default)]
pub struct GpuTopologyReport {
    pub adapters: usize,
    pub amd: usize,
    pub intel: usize,
    pub nvidia: usize,
    pub virtio: usize,
    pub other: usize,
    pub primary: Option<PciFunction>,
}

impl GpuTopologyReport {
    #[must_use]
    pub const fn native_vendor_count(&self) -> usize {
        self.amd + self.intel + self.nvidia
    }
}

#[must_use]
pub fn discover() -> GpuTopologyReport {
    let mut report = GpuTopologyReport::default();
    pci::enumerate(|function| {
        let virtio_gpu = function.vendor_id == VENDOR_VIRTIO
            && function.device_id == VIRTIO_GPU_MODERN_DEVICE;
        if function.class_code != PCI_CLASS_DISPLAY && !virtio_gpu {
            return;
        }
        report.adapters += 1;
        if report.primary.is_none() {
            report.primary = Some(function);
        }
        match function.vendor_id {
            VENDOR_AMD => report.amd += 1,
            VENDOR_INTEL => report.intel += 1,
            VENDOR_NVIDIA => report.nvidia += 1,
            VENDOR_VIRTIO if virtio_gpu => report.virtio += 1,
            _ => report.other += 1,
        }
    });
    report
}

pub fn self_test() -> Result<(), &'static str> {
    let sample = PciFunction {
        bus: 3,
        device: 0,
        function: 0,
        vendor_id: VENDOR_VIRTIO,
        device_id: VIRTIO_GPU_MODERN_DEVICE,
        class_code: PCI_CLASS_DISPLAY,
        subclass: 0,
        programming_interface: 0,
        revision: 1,
        header_type: 0,
    };
    if sample.vendor_id != VENDOR_VIRTIO || sample.class_code != PCI_CLASS_DISPLAY {
        return Err("GPU topology classifier self-test failed");
    }
    Ok(())
}
