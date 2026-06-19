/*
 * linker.ld — Linker script for RISC-V minimal kernel
 *
 * Controls where each section is placed in memory.
 * The CPU starts executing at 0x80000000 on QEMU virt.
 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
    /* Kernel is loaded at 0x80000000 (start of RAM on QEMU virt) */
    . = 0x80000000;

    /* Boot code must come first */
    .text : {
        *(.text.start)      /* _start entry point */
        *(.text .text.*)    /* All other code */
    }

    /* Read-only data */
    .rodata : {
        *(.rodata .rodata.*)
    }

    /* Initialized data */
    .data : {
        *(.data .data.*)
    }

    /* Uninitialized data (should be zeroed by bootloader) */
    .bss : {
        _bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        _bss_end = .;
    }

    /* Stack: 4 KB, placed after BSS, grows downward */
    . = ALIGN(16);
    _stack_bottom = .;
    . += 4096;              /* 4 KB stack */
    _stack_top = .;
}
