# =============================================================================
# RISC-V Assembly Test Programs for the 5-Stage Pipelined CPU
# Assemble with: riscv64-unknown-elf-as -march=rv32i programs.s -o programs.o
# Objcopy to hex: riscv64-unknown-elf-objcopy -O binary programs.o programs.bin
# Or generate hex with: riscv64-unknown-elf-objdump -d programs.o
# =============================================================================

.section .text
.globl _start

# -----------------------------------------------------------------------------
# Program 1: Fibonacci
# Compute fib(10) = 55 in x10 (a0)
# Exercises: ADD, BEQ, SW, LW, register arithmetic
# Expected: x10 = 55
# -----------------------------------------------------------------------------
fibonacci:
    addi x1, x0, 0       # a = 0
    addi x2, x0, 1       # b = 1
    addi x3, x0, 10      # n = 10
    addi x4, x0, 1       # i = 1

fib_loop:
    beq  x4, x3, fib_done # if i == n, done
    add  x5, x1, x2      # tmp = a + b
    add  x1, x2, x0      # a = b
    add  x2, x5, x0      # b = tmp
    addi x4, x4, 1       # i++
    beq  x0, x0, fib_loop # unconditional jump (BEQ with x0)

fib_done:
    add  x10, x2, x0     # result in a0 (should be 55)

# Store result to memory to verify
    addi x6, x0, 100     # base address
    sw   x10, 0(x6)      # mem[100] = 55
    lw   x7, 0(x6)       # x7 = 55 (verify load)

# -----------------------------------------------------------------------------
# Program 2: Factorial (iterative)
# Compute 6! = 720 in x10
# Exercises: ADDI, MUL-like loops (ADD-based), BEQ, BLT
# Expected: x10 = 720
# -----------------------------------------------------------------------------
factorial:
    addi x1, x0, 1       # result = 1
    addi x2, x0, 6       # n = 6
    addi x3, x0, 1       # i = 1

fact_loop:
    blt  x2, x3, fact_done  # if n < i, done
    # result *= i (using repeated addition)
    add  x4, x0, x0      # sum = 0
    add  x5, x1, x0      # copy of result
    add  x6, x3, x0      # copy of i

fact_mul:
    beq  x6, x0, fact_mul_done
    add  x4, x4, x5
    addi x6, x6, -1
    beq  x0, x0, fact_mul

fact_mul_done:
    add  x1, x4, x0      # result = product
    addi x3, x3, 1       # i++
    beq  x0, x0, fact_loop

fact_done:
    add  x11, x1, x0     # result in x11 (should be 720)

# Store factorial result
    addi x6, x0, 104
    sw   x11, 0(x6)      # mem[104] = 720

# -----------------------------------------------------------------------------
# Program 3: Bubble Sort
# Sort array [5, 3, 8, 1, 2] at address 200
# Exercises: LW, SW, BLT, branch hazards, load-use hazards
# Expected: mem[200..216] = {1, 2, 3, 5, 8}
# -----------------------------------------------------------------------------
bubblesort:
    # Initialize array in memory
    addi x1, x0, 200     # base address

    addi x2, x0, 5
    sw   x2, 0(x1)       # arr[0] = 5
    addi x2, x0, 3
    sw   x2, 4(x1)       # arr[1] = 3
    addi x2, x0, 8
    sw   x2, 8(x1)       # arr[2] = 8
    addi x2, x0, 1
    sw   x2, 12(x1)      # arr[3] = 1
    addi x2, x0, 2
    sw   x2, 16(x1)      # arr[4] = 2

    addi x3, x0, 5       # n = 5
    addi x4, x0, 0       # i = 0

outer_loop:
    addi x5, x3, -1      # n - 1
    beq  x4, x5, sort_done
    addi x6, x0, 0       # j = 0

inner_loop:
    sub  x7, x5, x4      # n - 1 - i
    beq  x6, x7, next_i

    # Load arr[j] and arr[j+1]
    sll  x8, x6, x2      # j * 4 (shift by 2)
    add  x8, x8, x1      # &arr[j]
    lw   x9, 0(x8)       # arr[j]          <-- load
    lw   x10, 4(x8)      # arr[j+1]        <-- load

    blt  x9, x10, no_swap # if arr[j] < arr[j+1], skip

    # Swap
    sw   x10, 0(x8)
    sw   x9, 4(x8)

no_swap:
    addi x6, x6, 1       # j++
    beq  x0, x0, inner_loop

next_i:
    addi x4, x4, 1       # i++
    beq  x0, x0, outer_loop

sort_done:
    # Verify: load sorted values
    lw   x12, 0(x1)      # should be 1
    lw   x13, 4(x1)      # should be 2
    lw   x14, 8(x1)      # should be 3
    lw   x15, 12(x1)     # should be 5
    lw   x16, 16(x1)     # should be 8

# -----------------------------------------------------------------------------
# Program 4: String Reverse (simplified — word-level)
# Reverse array of words in place at address 400
# Exercises: LW, SW, BLT, BEQ, address arithmetic
# Expected: mem[400..412] reversed
# -----------------------------------------------------------------------------
string_reverse:
    addi x1, x0, 400     # base address
    addi x2, x0, 4       # n-1 = 4 (5 elements, indices 0..4)

    # Initialize
    addi x3, x0, 65      # 'A'
    sw   x3, 0(x1)
    addi x3, x0, 66      # 'B'
    sw   x3, 4(x1)
    addi x3, x0, 67      # 'C'
    sw   x3, 8(x1)
    addi x3, x0, 68      # 'D'
    sw   x3, 12(x1)
    addi x3, x0, 69      # 'E'
    sw   x3, 16(x1)

    addi x4, x0, 0       # left = 0

rev_loop:
    blt  x2, x4, rev_done # if right < left, done

    # Load left and right
    sll  x5, x4, x1      # left * 4... (use shifts)
    # Simplified: manual offset calculation
    addi x5, x0, 0       # x5 will hold left offset
    beq  x4, x0, rev_skip_l0
    addi x5, x5, 4
rev_skip_l0:
    addi x6, x4, -1
    bne  x6, x0, rev_skip_l1
    addi x5, x5, 4
rev_skip_l1:
    # ... (in practice, use sll for index * 4)
    add  x5, x5, x1      # &arr[left]
    sll  x6, x2, x1      # right * 4... (simplified)
    # For brevity in assembly, we use direct offsets:
    # arr[left] at base + left*4, arr[right] at base + right*4

    # Simplified swap using known offsets
    beq  x4, x0, rev_swap_0
    addi x7, x0, 1
    beq  x4, x7, rev_swap_1
    beq  x0, x0, rev_done

rev_swap_0:
    # Swap arr[0] and arr[4]
    lw   x8, 0(x1)
    lw   x9, 16(x1)
    sw   x9, 0(x1)
    sw   x8, 16(x1)
    addi x4, x4, 1
    addi x2, x2, -1
    beq  x0, x0, rev_loop

rev_swap_1:
    # Swap arr[1] and arr[3]
    lw   x8, 4(x1)
    lw   x9, 12(x1)
    sw   x9, 4(x1)
    sw   x8, 12(x1)
    addi x4, x4, 1
    addi x2, x2, -1
    beq  x0, x0, rev_loop

rev_done:
    # Verify reversal
    lw   x17, 0(x1)      # should be 'E' = 69
    lw   x18, 4(x1)      # should be 'D' = 68
    lw   x19, 8(x1)      # should be 'C' = 67
    lw   x20, 12(x1)     # should be 'B' = 66
    lw   x21, 16(x1)     # should be 'A' = 65

# -----------------------------------------------------------------------------
# Halt (spin)
# -----------------------------------------------------------------------------
halt:
    beq  x0, x0, halt    # infinite loop — testbench watches for this
