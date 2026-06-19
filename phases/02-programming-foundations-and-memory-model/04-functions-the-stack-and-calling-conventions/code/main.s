# main.s — hand-written RISC-V (RV64) assembly for factorial.
#
# Demonstrates the RISC-V calling convention:
#   a0      : first argument (= n on entry) and return value
#   ra      : return address (caller-saved)
#   s0      : callee-saved register; we use it to preserve n across the recursive call
#   sp      : stack pointer; we allocate 16 bytes per frame (8 for ra, 8 for s0)
#
# Build (cross-compile or native RV64):
#   riscv64-linux-gnu-gcc -nostartfiles -static main.s -o factorial_rv
#
# This file is illustrative — it's the same factorial as main.c but in raw assembly.

    .text
    .globl factorial

# long factorial(long n)
factorial:
    # Base case: if n <= 1, return 1
    li    t0, 1
    ble   a0, t0, .Lbase

    # Prologue: save ra and s0; allocate stack frame
    addi  sp, sp, -16
    sd    ra, 8(sp)
    sd    s0, 0(sp)

    # Preserve n in s0 (callee-saved, so the recursive call won't clobber it)
    mv    s0, a0

    # Recursive call: factorial(n - 1)
    addi  a0, a0, -1
    jal   factorial            # ra <- next-instr; jump to factorial

    # On return, a0 = factorial(n - 1); we want n * a0
    mul   a0, s0, a0

    # Epilogue: restore ra, s0; pop frame
    ld    s0, 0(sp)
    ld    ra, 8(sp)
    addi  sp, sp, 16
    ret                        # jr ra

.Lbase:
    li    a0, 1                # return 1
    ret
