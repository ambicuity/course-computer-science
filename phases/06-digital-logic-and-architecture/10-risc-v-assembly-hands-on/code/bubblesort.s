# ─── bubblesort.s — In-place bubble sort of a word array ─────────
# Sort arr[0..n-1] in ascending order.
# C pseudocode:
#   for (i = 0; i < n-1; i++)
#     for (j = 0; j < n-1-i; j++)
#       if (arr[j] > arr[j+1]) swap(arr[j], arr[j+1])

        .data
arr:    .word   42, 17, 8, 91, 3, 55, 23, 6
n:      .word   8

        .text
        .globl main

# ── bubblesort function ───────────────────────────────────────────
# Entry:  a0 = base address of array
#         a1 = number of elements
# Return: array sorted in-place

bubblesort:
        addi    sp, sp, -16
        sw      ra, 12(sp)

        mv      t0, a0                 # t0 = array base (preserved)
        mv      t1, a1                 # t1 = n (preserved)

        li      t2, 0                   # t2 = i (outer loop counter)

outer:
        addi    t3, t1, -1             # t3 = n - 1
        bge     t2, t3, sort_done      # if i >= n-1, done

        li      t4, 0                   # t4 = j (inner loop counter)
        sub     t5, t1, t2             # t5 = n - i
        addi    t5, t5, -1             # t5 = n - i - 1 (inner limit)

inner:
        bge     t4, t5, inner_done     # if j >= n-i-1, inner loop done

        # Compute addresses: arr[j] and arr[j+1]
        slli    t6, t4, 2              # t6 = j * 4 (byte offset)
        add     a2, t0, t6            # a2 = &arr[j]
        lw      a3, 0(a2)             # a3 = arr[j]
        lw      a4, 4(a2)             # a4 = arr[j+1]

        # Compare and swap if arr[j] > arr[j+1]
        ble     a3, a4, no_swap        # if arr[j] <= arr[j+1], skip
        sw      a4, 0(a2)             # arr[j]   = arr[j+1]
        sw      a3, 4(a2)             # arr[j+1] = arr[j]

no_swap:
        addi    t4, t4, 1             # j++
        j       inner

inner_done:
        addi    t2, t2, 1             # i++
        j       outer

sort_done:
        lw      ra, 12(sp)
        addi    sp, sp, 16
        ret

# ── main ─────────────────────────────────────────────────────────
main:
        la      a0, arr                # a0 = base of arr
        lw      a1, n                  # a1 = 8
        jal     ra, bubblesort         # sort the array
        # arr is now: [3, 6, 8, 17, 23, 42, 55, 91]

        li      a7, 10
        ecall
