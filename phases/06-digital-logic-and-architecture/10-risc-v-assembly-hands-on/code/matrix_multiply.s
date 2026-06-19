# ─── matrix_multiply.s — Multiply two 3×3 matrices (row-major) ──
# C[i][j] = sum_k A[i][k] * B[k][j]
# All matrices stored as flat arrays of 9 words in row-major order.
#
# Memory layout for a 3×3 matrix M:
#   M[0][0] M[0][1] M[0][2] M[1][0] M[1][1] M[1][2] M[2][0] M[2][1] M[2][2]
#
# Entry:  a0 = address of A  (9 words)
#         a1 = address of B  (9 words)
#         a2 = address of C  (9 words, output)
# Uses nested i/j/k loops with index arithmetic.

        .data
# Matrix A = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
A:      .word   1, 2, 3, 4, 5, 6, 7, 8, 9
# Matrix B = [[9, 8, 7], [6, 5, 4], [3, 2, 1]]
B:      .word   9, 8, 7, 6, 5, 4, 3, 2, 1
C:      .word   0, 0, 0, 0, 0, 0, 0, 0, 0    # result (zeroed)

        .text
        .globl main

# ── matmul: C = A × B (3×3) ──────────────────────────────────────
matmul:
        addi    sp, sp, -32
        sw      ra, 28(sp)
        sw      s0, 24(sp)
        sw      s1, 20(sp)
        sw      s2, 16(sp)
        sw      s3, 12(sp)
        sw      s4,  8(sp)

        mv      s0, a0                 # s0 = &A
        mv      s1, a1                 # s1 = &B
        mv      s2, a2                 # s2 = &C

        li      s3, 0                   # s3 = i (row of A, row of C)

row_loop:
        li      t0, 3
        bge     s3, t0, matmul_done    # if i >= 3, done

        li      s4, 0                   # s4 = j (col of B, col of C)

col_loop:
        bge     s4, t0, next_row       # if j >= 3, next row

        # Compute C[i][j] = sum over k of A[i][k] * B[k][j]
        li      t1, 0                   # t1 = accumulator = 0
        li      t2, 0                   # t2 = k

k_loop:
        bge     t2, t0, store_c        # if k >= 3, store result

        # A[i][k] at offset (i*3 + k) * 4
        mul     t3, s3, t0             # t3 = i * 3
        add     t3, t3, t2            # t3 = i*3 + k
        slli    t3, t3, 2              # t3 = (i*3 + k) * 4 (byte offset)
        add     t3, s0, t3            # t3 = &A[i][k]
        lw      t3, 0(t3)             # t3 = A[i][k]

        # B[k][j] at offset (k*3 + j) * 4
        mul     t4, t2, t0             # t4 = k * 3
        add     t4, t4, s4            # t4 = k*3 + j
        slli    t4, t4, 2              # t4 = (k*3 + j) * 4
        add     t4, s1, t4            # t4 = &B[k][j]
        lw      t4, 0(t4)             # t4 = B[k][j]

        # Accumulate
        mul     t5, t3, t4            # t5 = A[i][k] * B[k][j]
        add     t1, t1, t5            # accum += product

        addi    t2, t2, 1             # k++
        j       k_loop

store_c:
        # C[i][j] = accumulator
        mul     t3, s3, t0             # t3 = i * 3
        add     t3, t3, s4            # t3 = i*3 + j
        slli    t3, t3, 2              # t3 = (i*3 + j) * 4
        add     t3, s2, t3            # t3 = &C[i][j]
        sw      t1, 0(t3)             # C[i][j] = accum

        addi    s4, s4, 1             # j++
        j       col_loop

next_row:
        addi    s3, s3, 1             # i++
        j       row_loop

matmul_done:
        lw      ra, 28(sp)
        lw      s0, 24(sp)
        lw      s1, 20(sp)
        lw      s2, 16(sp)
        lw      s3, 12(sp)
        lw      s4,  8(sp)
        addi    sp, sp, 32
        ret

# ── main ─────────────────────────────────────────────────────────
main:
        la      a0, A
        la      a1, B
        la      a2, C
        jal     ra, matmul
        # C = A × B = [[30,24,18],[84,69,54],[138,114,90]]

        li      a7, 10
        ecall
