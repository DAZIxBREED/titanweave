# K13.B Implementation Record — Modern VirtIO-GPU Transport

K13.B forks from the frozen, QEMU-qualified K13.A archive.

## Why this slice exists

K13.A proved that Titanweave can discover GPUs and expose backend-neutral memory,
queue, fence, modeset, and multi-GPU contracts without touching hardware. K13.B
is the first slice that turns one candidate into a live command transport.
VirtIO-GPU is used first because QEMU gives the project a deterministic test
adapter while the public ForgeGraphics contracts remain vendor-neutral.

## Transport ownership sequence

The K13.B VirtIO backend follows this order:

1. rediscover the exact VirtIO-GPU PCI function;
2. claim its ForgeBus device with the exact `1af4:1050` driver match;
3. establish a bounded ForgeBus DMA ownership domain;
4. enable PCI memory decoding only;
5. walk modern VirtIO PCI capabilities;
6. reset the device and negotiate `VIRTIO_F_VERSION_1`;
7. allocate/publish split control and cursor virtqueues;
8. allocate a bounded command workspace;
9. enable PCI bus mastering;
10. enter `DRIVER_OK`;
11. issue real GPU protocol commands;
12. mark the ForgeBus device online only after scanout transfer/flush succeeds.

This ordering ensures that bus mastering never precedes driver ownership and DMA
bookkeeping.

## Commands qualified in K13.B

The bootstrap control queue issues:

- `VIRTIO_GPU_CMD_GET_DISPLAY_INFO`;
- `VIRTIO_GPU_CMD_RESOURCE_CREATE_2D`;
- `VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING`;
- `VIRTIO_GPU_CMD_SET_SCANOUT`;
- `VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D`;
- `VIRTIO_GPU_CMD_RESOURCE_FLUSH`.

The backing buffer is filled with a deterministic Titanweave dark/purple test
pattern. K12's GOP preview is not removed; the VirtIO output is a separate K13.B
transport proof.

## Completion model

K13.B uses polled used-ring completion during early boot. This avoids making MSI
or interrupt migration a prerequisite for the first live transport. The queue
is still a reusable split virtqueue; MSI/fence-driven asynchronous completion is
planned for K13.C when DISPLAYD begins presenting continuously.

## Recovery model

A timed-out command sets/reset the VirtIO device and disables PCI bus mastering.
ForgeBus owns the DMA allocations, allowing the watchdog/recovery architecture to
revoke them as a unit. Full resource teardown on live driver restart is a K13.C
hardening task.
