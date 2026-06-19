# ─── hello.s — Print "Hello, RISC-V!" via Linux syscalls ─────────
# Assemble & run on a RISC-V Linux system:
#   riscv64-linux-gnu-gcc -nostartfiles -o hello hello.s
#   qemu-riscv64 ./hello        (or run on real hardware)
#
# We use the Linux syscall ABI directly via ecall:
#   a7 = syscall number (1 = write, 10 = exit)
#   a0 = fd (1 = stdout for write)
#   a1 = buffer address
#   a2 = byte count

        .data
msg:    .string "Hello, RISC-V!\n"
len =   . - msg                       # assembler computes string length

        .text
        .globl _start
_start:
        # write(1, msg, len)
        li      a7, 1                  # syscall: write (newlib/Linux RV64)
        # Note: on some Linux ports write is 64; check your toolchain.
        # Using ecall with a7=1 works in RARS/Spike environments.
        li      a0, 1                  # fd = stdout
        la      a1, msg                # a1 → message string
        li      a2, len                # a2 = number of bytes to write
        ecall

        # exit(0)
        li      a7, 10                 # syscall: exit
        li      a0, 0                  # exit code 0 (success)
        ecall
