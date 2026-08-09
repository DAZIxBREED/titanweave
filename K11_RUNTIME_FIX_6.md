# K11 Runtime Fix 6 — Reserve syscall vector 0x80

The first ring-3 instruction sequence reached `int 0x80` and raised #GP with
error code `0x402`. In long mode, that error identifies IDT vector 0x80
(`0x80 * 8 + IDT-bit 2`).

K11 correctly created vector 0x80 as a DPL=3 software interrupt gate, but its
device-vector population loop covered 0x50 through 0xdf and ran afterwards,
replacing vector 0x80 with a DPL=0 device interrupt gate. Every native service
therefore faulted on its first syscall.

This fix reserves vector 0x80 in both places that own interrupt-vector policy:

- IDT device-gate population skips the syscall vector and explicitly finalizes
  vector 0x80 as the DPL=3 syscall gate after device gates are installed.
- `InterruptRouter` removes one route from the device-vector capacity and maps
  route slots around the 0x80 hole, preventing MSI/MSI-X/device allocation from
  ever leasing the syscall ABI vector.
- Semantic regression checks enforce both invariants.
