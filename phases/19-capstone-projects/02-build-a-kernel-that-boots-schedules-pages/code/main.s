# Build a Kernel That Boots, Schedules, Pages — RISC-V Boot Entry
# Run: riscv64-unknown-elf-gcc -nostdlib -T kernel.ld boot.S scheduler.c vm.c kmain.c -o kernel.elf
# Run: qemu-system-riscv64 -machine virt -kernel kernel.elf -nographic
#
# This is the RISC-V assembly boot entry that:
#   1. Disables interrupts
#   2. Sets up the kernel stack
#   3. Clears the BSS section
#   4. Jumps to kmain() in C
#   5. Halts if kmain returns

# boot.S — RISC-V boot entry
.section .text.init
.global _start

_start:
    # Disable interrupts
    csrw sie, zero
    csrw sip, zero

    # Set up stack pointer
    la   sp, _stack_top

    # Clear BSS section
    la   t0, _bss_start
    la   t1, _bss_end
clear_bss:
    bge  t0, t1, bss_done
    sd   zero, 0(t0)
    addi t0, t0, 8
    j    clear_bss
bss_done:

    # Jump to C kernel entry
    call kmain

    # If kmain returns, spin forever
spin:
    wfi
    j    spin

.section .bss
.align 12
_stack_bottom:
    .space 4096 * 4    # 16 KB kernel stack
_stack_top:

# ============================================================================
# trap.S — Trap entry/exit (save/restore all 31 GP registers + sepc)
# ============================================================================

.global trap_entry
.global trap_exit

.align 4
trap_entry:
    # Save all 31 general-purpose registers to the trap frame
    # (stored at the address in tp, which points to current task's context)
    sd   ra,  0*8(tp)
    sd   sp,  1*8(tp)
    sd   gp,  2*8(tp)
    # tp is reserved for the trap frame pointer
    sd   t0,  4*8(tp)
    sd   t1,  5*8(tp)
    sd   t2,  6*8(tp)
    sd   s0,  7*8(tp)
    sd   s1,  8*8(tp)
    sd   a0,  9*8(tp)
    sd   a1, 10*8(tp)
    sd   a2, 11*8(tp)
    sd   a3, 12*8(tp)
    sd   a4, 13*8(tp)
    sd   a5, 14*8(tp)
    sd   a6, 15*8(tp)
    sd   a7, 16*8(tp)
    sd   s2, 17*8(tp)
    sd   s3, 18*8(tp)
    sd   s4, 19*8(tp)
    sd   s5, 20*8(tp)
    sd   s6, 21*8(tp)
    sd   s7, 22*8(tp)
    sd   s8, 23*8(tp)
    sd   s9, 24*8(tp)
    sd  s10, 25*8(tp)
    sd  s11, 26*8(tp)
    sd   t3, 27*8(tp)
    sd   t4, 28*8(tp)
    sd   t5, 29*8(tp)
    sd   t6, 30*8(tp)

    # Save sepc (the interrupted PC)
    csrr t0, sepc
    sd   t0, 31*8(tp)

    # Call C trap handler
    call trap_handler

trap_exit:
    # Restore sepc
    ld   t0, 31*8(tp)
    csrw sepc, t0

    # Restore all registers
    ld   ra,  0*8(tp)
    ld   sp,  1*8(tp)
    ld   gp,  2*8(tp)
    ld   t0,  4*8(tp)
    ld   t1,  5*8(tp)
    ld   t2,  6*8(tp)
    ld   s0,  7*8(tp)
    ld   s1,  8*8(tp)
    ld   a0,  9*8(tp)
    ld   a1, 10*8(tp)
    ld   a2, 11*8(tp)
    ld   a3, 12*8(tp)
    ld   a4, 13*8(tp)
    ld   a5, 14*8(tp)
    ld   a6, 15*8(tp)
    ld   a7, 16*8(tp)
    ld   s2, 17*8(tp)
    ld   s3, 18*8(tp)
    ld   s4, 19*8(tp)
    ld   s5, 20*8(tp)
    ld   s6, 21*8(tp)
    ld   s7, 22*8(tp)
    ld   s8, 23*8(tp)
    ld   s9, 24*8(tp)
    ld  s10, 25*8(tp)
    ld  s11, 26*8(tp)
    ld   t3, 27*8(tp)
    ld   t4, 28*8(tp)
    ld   t5, 29*8(tp)
    ld   t6, 30*8(tp)

    sret
