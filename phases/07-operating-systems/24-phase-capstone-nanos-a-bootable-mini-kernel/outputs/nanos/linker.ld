OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
    . = 0x80200000;

    .text : {
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    _bss_start = .;
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
    _bss_end = .;

    . = ALIGN(4096);
    _heap_start = .;
    . += 0x100000;       /* 1 MB heap */
    _heap_end = .;

    _stack_top = .;
}
