# Titanweave K14.C29 Runtime Qualification

Status: **QUALIFIED / FROZEN**

Fedora/QEMU runtime qualification passed on 2026-08-09.

Passed gates:

- `[BOOT]`
- `[C28OK]`
- `[C29RG]`
- `[C29QU]`
- `[C29FN]`
- `[C29DM]`
- `[C29SD]`
- `[C29PG]`
- `[C29RD]`
- `[C29OK]`
- `[USER]`
- `[RECV]`
- `[K14FOUND]`
- `[KERN]`
- `[QUAL]`
- `[HALT]`

QEMU terminated normally after Titanweave emitted its intentional HALT marker with raw exit status 0.

K14.C29 rings, queues, fences, and DMA is QUALIFIED / FROZEN.

Physical Radeon SDMA execution remains a separate bare-metal qualification; QEMU qualified the GTT ring, FIFO queue, typed SDMA packet codec, timeline fence, and owned-memory DMA execution path.
