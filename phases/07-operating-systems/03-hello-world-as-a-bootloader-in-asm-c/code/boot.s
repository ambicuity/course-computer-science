# boot.s — RISC-V bootloader
#
# This is the first code the CPU executes.
# It sets up the stack and calls the C kernel entry point.

    .section .text.start
    .globl _start

_start:
    # Set stack pointer to top of stack (defined in linker script)
    la      sp, _stack_top

    # Call kernel_main() in C
    call    kernel_main

    # If kernel_main returns, halt the CPU in an infinite loop
halt:
    wfi                         # Wait For Interrupt (low power)
    j       halt                # Loop forever
