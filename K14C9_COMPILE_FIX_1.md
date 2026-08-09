# K14.C9 compile fix 1

Fixes two type/identity mistakes in the C9 live PCI verification path:

- `NativeBindingState.selected_vendor` is a `NativeGpuVendor` tag stored as `u8`, not a PCI vendor ID. C9 now compares it with `NativeGpuVendor::Amd as u8`.
- `NativeBindingState.selected_device` is the PCI slot/device number (0..31), not the 16-bit PCI device ID. C9 now compares the live PCI vendor/device/revision tuple against the identity captured by C7/C8 (`C8State.vendor_id/device_id/revision`).

This avoids both the Rust E0308 errors and the more serious possibility of making a logically invalid identity comparison merely by casting the slot number to `u16`.
