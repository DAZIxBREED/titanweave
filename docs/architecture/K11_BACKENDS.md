# K11.1-K11.8 Hardware Backend Integration

This revision adds the common IOMMU/DMA core and concrete backend implementations for the K11 hardware sequence.

- K11.1 AMD-Vi: IVRS/IVHD discovery, default-deny requester state, command ring, domain attach/detach, mapping and invalidation contract.
- K11.2 Intel VT-d: DMAR/DRHD discovery, default-deny requester state, queued invalidation and domain operations.
- K11.3 MSI/MSI-X: PCI capability walking, message construction and lease lifecycle.
- K11.4 xHCI: controller validation, command/event transfer rings, reset/start and control-transfer construction.
- K11.5 USB HID: persistent keyboard/mouse interface state and report decoding.
- K11.6 NVMe: admin and I/O queue model, namespaces, read/write/flush/discard commands, completion and reset handling.
- K11.7 PCIe hot-plug: slot registration, debounce, generation-safe insertion and surprise-removal handling.
- K11.8 Stress: deterministic IOVA, ring saturation, stale-generation and HID malformed-report tests.

The common preparation also adds a validated ACPI catalog, MCFG/ECAM windows, segment-aware PCI requester identities, IOVA allocation, translated page mappings and fence tokens.

Hardware register activation still requires compilation and qualification on AMD and Intel machines. The runtime defaults to denying external DMA when no supported IOMMU table is present.
