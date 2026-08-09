OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

USER_BASE = 0x0000010000000000;

PHDRS
{
    text PT_LOAD FLAGS(5);
    data PT_LOAD FLAGS(6);
}

SECTIONS
{
    . = USER_BASE;
    .text ALIGN(0x1000) : { *(.text .text.*) } :text
    .rodata ALIGN(0x1000) : { *(.rodata .rodata.*) } :text
    .data ALIGN(0x1000) : { *(.data .data.*) } :data
    .bss ALIGN(0x1000) (NOLOAD) : { *(.bss .bss.*) *(COMMON) } :data
    /DISCARD/ : { *(.eh_frame*) *(.comment*) *(.note*) }
}
