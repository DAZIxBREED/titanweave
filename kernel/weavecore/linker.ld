OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)
ENTRY(weavecore_entry)

KERNEL_VIRTUAL_BASE = 0xffffffff80000000;
KERNEL_PHYSICAL_BASE = 0x02000000;

PHDRS
{
    kernel PT_LOAD FLAGS(7);
}

SECTIONS
{
    . = KERNEL_VIRTUAL_BASE;
    __kernel_virtual_start = .;
    __kernel_physical_start = KERNEL_PHYSICAL_BASE;

    .text ALIGN(0x1000) : AT(KERNEL_PHYSICAL_BASE + (ADDR(.text) - KERNEL_VIRTUAL_BASE))
    {
        KEEP(*(.text.entry))
        *(.text .text.*)
    } :kernel

    .ap_trampoline ALIGN(0x1000) : AT(KERNEL_PHYSICAL_BASE + (ADDR(.ap_trampoline) - KERNEL_VIRTUAL_BASE))
    {
        KEEP(*(.ap_trampoline))
    } :kernel

    .rodata ALIGN(0x1000) : AT(KERNEL_PHYSICAL_BASE + (ADDR(.rodata) - KERNEL_VIRTUAL_BASE))
    {
        *(.rodata .rodata.*)
    } :kernel

    /* Keep rust-lld's synthesized GOT out of the image origin/.text. */
    .got ALIGN(0x1000) : AT(KERNEL_PHYSICAL_BASE + (ADDR(.got) - KERNEL_VIRTUAL_BASE))
    {
        *(.got .got.*)
        *(.got.plt)
    } :kernel

    .data ALIGN(0x1000) : AT(KERNEL_PHYSICAL_BASE + (ADDR(.data) - KERNEL_VIRTUAL_BASE))
    {
        *(.data .data.*)
    } :kernel

    .bss ALIGN(0x1000) (NOLOAD) : AT(KERNEL_PHYSICAL_BASE + (ADDR(.bss) - KERNEL_VIRTUAL_BASE))
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } :kernel

    . = ALIGN(0x1000);
    __kernel_virtual_end = .;
    __kernel_physical_end = KERNEL_PHYSICAL_BASE + (__kernel_virtual_end - KERNEL_VIRTUAL_BASE);

    /DISCARD/ :
    {
        *(.eh_frame*)
        *(.comment*)
        *(.note*)
        /* Do not embed non-alloc DWARF in the fixed-load boot ELF. */
        *(.debug*)
        *(.zdebug*)
        *(.gdb_index)
    }
}
