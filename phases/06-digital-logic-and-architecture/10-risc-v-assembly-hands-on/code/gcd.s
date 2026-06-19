# ─── gcd.s — Recursive GCD using Euclidean algorithm ─────────────
# gcd(a, b):
#   if b == 0  →  return a
#   else       →  return gcd(b, a mod b)
#
# Follows the full RISC-V calling convention:
#   - a0 = first argument,  a1 = second argument
#   - Return value in a0
#   - Save/restore ra and callee-saved registers on the stack
#
# Test with: rars, Spike, or any RV32I/RV64I simulator

        .text
        .globl main

# ── gcd function ──────────────────────────────────────────────────
# Entry:  a0 = a,  a1 = b
# Return: a0 = gcd(a, b)

gcd:
        # Base case: if b == 0, return a
        beqz    a1, gcd_done           # if b == 0, jump to done

        # Recursive case: gcd(b, a % b)
        # Save ra and a0 on the stack (we need them after the recursive call)
        addi    sp, sp, -16            # allocate 4 words on stack
        sw      ra, 12(sp)             # save return address
        sw      a0,  8(sp)             # save original a
        sw      a1,  4(sp)             # save original b

        # Set up args for gcd(b, a % b)
        lw      t0,  8(sp)             # t0 = original a
        lw      t1,  4(sp)             # t1 = original b

        mv      a0, t1                 # a0 = b  (new first arg)
        rem     a1, t0, t1             # a1 = a % b (new second arg)

        jal     ra, gcd                # a0 = gcd(b, a % b)

        # Restore and return
        lw      ra, 12(sp)             # restore return address
        addi    sp, sp, 16             # deallocate stack frame
        ret

gcd_done:
        # a0 already holds 'a' — it is the answer
        ret

# ── main — test gcd ──────────────────────────────────────────────
main:
        # Compute gcd(48, 18) — expect 6
        li      a0, 48
        li      a1, 18
        jal     ra, gcd
        # a0 = 6

        # Compute gcd(1071, 462) — expect 21
        li      a0, 1071
        li      a1, 462
        jal     ra, gcd
        # a0 = 21

        # exit
        li      a7, 10
        ecall
