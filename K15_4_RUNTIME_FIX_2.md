# K15.4 Runtime Fix 2 — HDA MSI interrupt window

Fedora/QEMU reached real HDA stream DMA and reported `SDnSTS=0x24` with LPIB progress but no Titanweave MSI handler dispatch. In QEMU HDA, `0x24` is FIFO-ready (`0x20`) plus buffer-completion status (`0x04`), proving the BDL IOC completion condition occurred.

The missing dispatch was caused by CPU interrupt state. K15.1 intentionally disables local interrupts after its RT self-test; K15.4 then waited for a real device MSI while IF remained clear.

The fix:

- verifies HDA `INTCTL.GIE` and the selected stream interrupt bit before RUN;
- records the pre-wait RFLAGS/IF state;
- enables local interrupts only for the bounded HDA MSI wait when IF was previously clear;
- requires `STREAM_IRQ_EVENTS` to advance through the actual HDA handler;
- restores IF to its exact previous disabled/enabled state before continuing boot;
- expands timeout evidence with SD status/LPIB, stream bit, `INTCTL`, `INTSTS`, and IF state.

The K15.3 transport is still retired only after hardware interrupt evidence. No polling fallback or synthetic completion was added.
