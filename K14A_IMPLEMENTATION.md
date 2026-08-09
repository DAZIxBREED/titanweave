# K14.A Implementation Record

K14.A establishes the native-GPU safety contract on top of frozen K13.D.

The native probe reads PCI identity, command/status and BAR addresses only. It
must not call PCI configuration writes, ForgeBus claim APIs, kernel MMIO mapping,
or bus-master enabling functions. The static K14.A validator checks this
boundary.

Native bus mastering is admitted only when both conditions are true:

1. the device has been explicitly claimed by ForgeBus; and
2. the IOMMU state is `HardwareTranslated`.

The current K11 Intel VT-d/AMD-Vi code is intentionally classified as
`PolicyOnly`; it discovers firmware tables and models default-deny state but
does not yet program real hardware translation tables. Therefore K14.A always
keeps native activation deferred while retaining K13 VirtIO-GPU and K12 GOP.
