# ─── RISC-V Assembly Programs ─────────────────────────────────────
# Five programs demonstrating core RV32I patterns.
# Assemble with: riscv64-unknown-elf-gcc -march=rv32i -mabi=ilp32 programs.s
# These target RARS/Spike or any RV32I emulator.

        .data
nl:     .string "\n"
str:    .string "abcde"                  # for string_length test
arr:    .word   3, 7, 1, 9, 4, 6, 2     # for array_sum test

        .text
        .globl main

# ═══════════════════════════════════════════════════════════════════
#  1. sum_1_to_n — Compute the sum 1 + 2 + … + N
#     a0 (input) = N   a0 (output) = N*(N+1)/2
# ═══════════════════════════════════════════════════════════════════

sum_1_to_n:
        li      a0, 10                  # N = 10
        li      t0, 0                   # t0 = accumulator, starts at 0
        li      t1, 1                   # t1 = counter i, starts at 1
loop1:
        bgt     t1, a0, done1           # if i > N, exit loop
        add     t0, t0, t1             # accum += i
        addi    t1, t1, 1              # i++
        j       loop1                  # repeat
done1:
        mv      a0, t0                 # return result in a0
        # a0 = 55 when N=10
        ret

# ═══════════════════════════════════════════════════════════════════
#  2. factorial — Recursive factorial(n)
#     a0 (input) = n   a0 (output) = n!
# ═══════════════════════════════════════════════════════════════════

factorial:
        addi    sp, sp, -8              # allocate 2 words on stack
        sw      ra, 4(sp)              # save return address
        sw      a0, 0(sp)              # save n

        # base case: n <= 1 → return 1
        li      t0, 1
        ble     a0, t0, base

        # recursive case: n * factorial(n-1)
        addi    a0, a0, -1             # a0 = n-1
        jal     ra, factorial           # a0 = factorial(n-1)
        lw      t0, 0(sp)              # restore original n
        mul     a0, a0, t0            # a0 = factorial(n-1) * n
        j       ret1

base:
        li      a0, 1                  # return 1

ret1:
        lw      ra, 4(sp)              # restore return address
        addi    sp, sp, 8              # deallocate stack frame
        ret                            # jr ra

# ═══════════════════════════════════════════════════════════════════
#  3. fibonacci — Iterative fib(n)
#     a0 (input) = n   a0 (output) = fib(n)
# ═══════════════════════════════════════════════════════════════════

fibonacci:
        li      a0, 10                 # compute fib(10)
        li      t0, 0                   # t0 = fib(i-2) = fib(0) = 0
        li      t1, 1                   # t1 = fib(i-1) = fib(1) = 1
        li      t2, 2                   # t2 = loop counter i, starts at 2

        # fib(0) = 0, fib(1) = 1
        blt     a0, t2, fib_base

fib_loop:
        bgt     t2, a0, fib_done       # if i > n, done
        add     t3, t0, t1            # t3 = fib(i) = fib(i-1) + fib(i-2)
        mv      t0, t1                 # shift: old fib(i-1) → fib(i-2)
        mv      t1, t3                 # shift: new fib(i)   → fib(i-1)
        addi    t2, t2, 1             # i++
        j       fib_loop

fib_base:
        mv      a0, t0                 # for n=0, return 0; for n=1 this is wrong
        bnez    a0, fib_n1
        ret                            # n=0 → return 0
fib_n1:
        li      a0, 1                  # n=1 → return 1
        ret

fib_done:
        mv      a0, t1                 # result in t1 → a0
        # a0 = 55 when n=10
        ret

# ═══════════════════════════════════════════════════════════════════
#  4. string_length — Count characters in a null-terminated string
#     a0 = pointer to string   a0 (output) = length
# ═══════════════════════════════════════════════════════════════════

string_length:
        la      a0, str                # a0 → "abcde"
        li      t0, 0                   # t0 = length counter = 0

strlen_loop:
        lb      t1, 0(a0)             # load byte at address a0
        beqz    t1, strlen_done        # if null terminator, stop
        addi    t0, t0, 1             # length++
        addi    a0, a0, 1             # advance pointer to next byte
        j       strlen_loop

strlen_done:
        mv      a0, t0                 # return length
        # a0 = 5 for "abcde"
        ret

# ═══════════════════════════════════════════════════════════════════
#  5. array_sum — Sum all elements of a word array
#     a0 = array base address, a1 = element count
#     a0 (output) = sum
# ═══════════════════════════════════════════════════════════════════

array_sum:
        la      a0, arr                # a0 = base of arr
        li      a1, 7                   # a1 = number of elements
        li      t0, 0                   # t0 = running sum
        li      t1, 0                   # t1 = byte offset

arr_loop:
        beqz    a1, arr_done           # if no elements left, done
        lw      t2, 0(a0)             # load word at a0 + 0
        add     t0, t0, t2            # sum += arr[i]
        addi    a0, a0, 4             # advance by 4 bytes (one word)
        addi    a1, a1, -1            # element_count--
        j       arr_loop

arr_done:
        mv      a0, t0                 # return sum
        # a0 = 32 for [3,7,1,9,4,6,2]
        ret

# ═══════════════════════════════════════════════════════════════════
#  main — run all five programs in sequence (for testing)
# ═══════════════════════════════════════════════════════════════════

main:
        # call sum_1_to_n
        jal     ra, sum_1_to_n
        # a0 = 55

        # call factorial(10)
        li      a0, 10
        jal     ra, factorial
        # a0 = 3628800

        # call fibonacci(10)
        jal     ra, fibonacci
        # a0 = 55

        # call string_length("abcde")
        jal     ra, string_length
        # a0 = 5

        # call array_sum
        jal     ra, array_sum
        # a0 = 32

        # exit via ecall
        li      a7, 10                  # syscall number for exit
        ecall                          # terminate
