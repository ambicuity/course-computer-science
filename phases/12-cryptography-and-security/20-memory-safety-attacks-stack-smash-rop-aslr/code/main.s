# RISC-V Stack Overflow Demo
# Phase 12 — Cryptography & Security, Lesson 20
#
# Demonstrates buffer overflow overwriting the ra (return address) register
# on a RISC-V architecture. Shows function prologue/epilogue, calling
# convention (ra saved/restored), and control-flow hijacking.
#
# RISC-V Calling Convention (RV64):
#   ra   = return address (set by jal/jalr, consumed by ret)
#   sp   = stack pointer (grows downward)
#   s0   = frame pointer (saved frame pointer, callee-saved)
#   a0-a7 = function arguments
#   a0-a1 = return values
#   s1-s11 = callee-saved registers
#
# Stack layout in vulnerable() after prologue:
#   Higher addresses
#   +-----------------------+
#   | caller's stack frame  |
#   +-----------------------+
#   | saved ra (8 bytes)    |  <- sp+72  <- OVERWRITTEN by overflow
#   +-----------------------+
#   | saved s0 (8 bytes)    |  <- sp+64
#   +-----------------------+
#   | buf[63..56]           |
#   | ... (64 bytes total)  |
#   | buf[7..0]             |  <- sp+0 (= sp, buffer start)
#   +-----------------------+  <- sp after prologue
#   Lower addresses
#
# To exploit: provide 72 bytes of padding + 8-byte address of win().
# The 73rd byte overwrites the least significant byte of saved ra.
#
# Compile (requires riscv64-linux-gnu-gcc or host RISC-V GCC):
#   riscv64-linux-gnu-gcc -static -o exploit main.s
# Run with QEMU:
#   qemu-riscv64 ./exploit
# Provide 80 bytes of input: 72 'A's + 8 bytes of win() address (little-endian).

.section .rodata
msg_prompt:       .string "Enter input (RISC-V stack overflow demo): "
msg_win:          .string "\n*** WIN! Control hijacked via RISC-V stack overflow! ***\n"
msg_normal:       .string "Returned normally.\n"
msg_corrupted:    .string "Returned abnormally (ra was hijacked?)\n"
msg_gets_prompt:  .string "vulnerable_gets: reading into buf[64] at sp+0\n"
msg_strcpy_label: .string "vulnerable_strcpy: copying into buf[64] at sp+0\n"
msg_layout_str:
    .string "=== RISC-V Stack Layout ===\n"
    .string "buf[64]           at 0x%016lx\n"
    .string "saved s0          at 0x%016lx\n"
    .string "saved ra          at 0x%016lx\n"
    .string "offset (buf->ra): %ld bytes\n"
    .string "win() at          0x%016lx\n"
    .string "Input beyond %ld bytes overwrites ra\n"
    .string "\n"
msg_usage:
    .string "This program demonstrates a RISC-V stack buffer overflow.\n"
    .string "Enter more than 72 bytes to overwrite the saved ra register.\n"
    .string "If you overwrite ra with the address of win(), execution\n"
    .string "will jump there instead of returning to main().\n"
    .string "\n"
msg_bye:          .string "main() exiting normally.\n"

.section .bss
.align 4
input_buf:       .skip 256

.section .text
.globl main
.type main, @function
main:
    # Function prologue: save callee-saved registers
    # main saves ra and s0 because it calls other functions.
    addi sp, sp, -16
    sd   ra, 8(sp)           # Save return address to main's caller
    sd   s0, 0(sp)           # Save frame pointer
    addi s0, sp, 16          # Set frame pointer (points to saved ra)

    # Print header and usage
    la   a0, msg_prompt
    jal  printf

    la   a0, msg_usage
    jal  printf

    # --- Demo 1: vulnerable_gets() ---
    # This function has a local buffer and calls gets().
    # Overflowing it will overwrite the saved ra.
    la   a0, msg_gets_prompt
    jal  printf

    jal  vulnerable_gets

    # Check if we returned normally or got hijacked.
    # If ra was overwritten and we returned to win(), we won't reach here.
    # If ra was overwritten with garbage, we'd crash (SIGSEGV).
    # If we're here, the function returned normally.
    la   a0, msg_normal
    jal  printf

    # --- Demo 2 (commented): vulnerable_strcpy() ---
    # Same concept but uses strcpy instead of gets.
    # The danger is the same: unbounded copy into a local buffer.
    # jal vulnerable_strcpy
    # la   a0, msg_normal
    # jal  printf

    # Epilogue: restore saved registers and return
    la   a0, msg_bye
    jal  printf

    ld   s0, 0(sp)
    ld   ra, 8(sp)
    addi sp, sp, 16
    li   a0, 0               # Return 0 from main
    ret


# win — target function that the attacker wants to call
# This function is never called directly by the program.
# It is reached only by hijacking the return address.
.type win, @function
win:
    # Prologue: save ra (since we call printf)
    addi sp, sp, -16
    sd   ra, 8(sp)

    la   a0, msg_win
    jal  printf

    # Epilogue
    ld   ra, 8(sp)
    addi sp, sp, 16
    ret


# vulnerable_gets — function with a buffer overflow
# Allocates 64 bytes for a local buffer, calls gets() to fill it.
# The saved ra is at sp+72 — 72 bytes above the buffer.
# Input >72 bytes overwrites ra, hijacking the ret instruction.
.type vulnerable_gets, @function
vulnerable_gets:
    # Prologue:
    # Allocate 80 bytes: 64 (buf) + 8 (saved s0) + 8 (saved ra)
    # sp+0  to sp+63: buf[64]  (local buffer)
    # sp+64 to sp+71: saved s0 (frame pointer, restored on epilogue)
    # sp+72 to sp+79: saved ra (return address — TARGET FOR OVERFLOW)
    addi sp, sp, -80
    sd   ra, 72(sp)           # Save return address at sp+72
    sd   s0, 64(sp)           # Save frame pointer at sp+64
    addi s0, sp, 80           # Set frame pointer

    # Print stack layout for educational purposes
    # a1 = buf address (sp+0)
    # a2 = saved s0 address (sp+64)
    # a3 = saved ra address (sp+72)
    # a4 = offset (72 = sp+72 - sp+0)
    # a5 = win() address
    addi a1, sp, 0
    addi a2, sp, 64
    addi a3, sp, 72
    li   a4, 72
    la   a5, win
    la   a0, msg_layout_str
    jal  printf

    # Print the "Enter input" prompt
    la   a0, msg_prompt
    jal  printf

    # gets(buf) — read input into local buffer
    # UNPROTECTED: no bounds checking. Input >72 bytes overwrites ra.
    addi a0, sp, 0            # a0 = &buf[0]
    jal  gets

    # Epilogue: restore saved registers and return
    ld   s0, 64(sp)           # Restore frame pointer
    ld   ra, 72(sp)           # Restore return address (CORRUPTED if overflow!)
    addi sp, sp, 80           # Deallocate stack frame

    # ret is a pseudo-instruction for: jalr zero, ra, 0
    # If ra was overwritten, this jumps to the attacker-controlled address.
    ret


# vulnerable_strcpy — same concept via strcpy
# Shows that the vulnerability is not specific to gets().
# Any function that writes without bounds into a local buffer is dangerous.
.type vulnerable_strcpy, @function
vulnerable_strcpy:
    # Prologue: same layout as vulnerable_gets
    addi sp, sp, -80
    sd   ra, 72(sp)
    sd   s0, 64(sp)
    addi s0, sp, 80

    # Print label
    la   a0, msg_strcpy_label
    jal  printf

    # strcpy(buf, input_buf) — copy from global input buffer
    # If input_buf contains more than 72 bytes, ra is overwritten.
    la   a1, input_buf
    addi a0, sp, 0
    jal  strcpy

    # Epilogue
    ld   s0, 64(sp)
    ld   ra, 72(sp)           # CORRUPTED if input_buf >72 bytes!
    addi sp, sp, 80
    ret


# Global input buffer reader
# Reads a line from stdin into input_buf (max 255 bytes).
# This simulates reading attacker-controlled data into a global.
.type read_input, @function
read_input:
    addi sp, sp, -16
    sd   ra, 8(sp)

    # fgets(input_buf, 255, stdin)
    la   a0, input_buf
    li   a1, 255
    ld   a2, stdin
    jal  fgets

    ld   ra, 8(sp)
    addi sp, sp, 16
    ret


# Helper to indicate that .note.GNU-stack is non-executable
.section .note.GNU-stack,"",@progbits
