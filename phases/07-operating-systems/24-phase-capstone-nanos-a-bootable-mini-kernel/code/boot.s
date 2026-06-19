/*
 * boot.s — RISC-V 64-bit entry point for nanos
 *
 * The bootloader (or QEMU's built-in firmware) jumps here.
 * We set up the stack, zero BSS, and call kernel_main().
 */

.section .text
.global _start

_start:
    /* Set up the stack pointer (defined by linker script) */
    la sp, _stack_top

    /* Zero the BSS section (_bss_start .. _bss_end) */
    la t0, _bss_start
    la t1, _bss_end
_zero_bss:
    bge t0, t1, _bss_done
    sd zero, 0(t0)
    addi t0, t0, 8
    j _zero_bss
_bss_done:

    /* Jump into the C kernel */
    call kernel_main

    /* If kernel_main ever returns, spin forever */
_halt:
    wfi
    j _halt
