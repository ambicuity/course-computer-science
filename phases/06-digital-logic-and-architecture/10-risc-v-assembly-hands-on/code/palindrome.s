# ─── palindrome.s — Check if a string is a palindrome ────────────
# A palindrome reads the same forwards and backwards (e.g., "racecar").
# Algorithm:
#   1. Find string length.
#   2. Set left pointer to start, right pointer to end.
#   3. Compare bytes; if any differ, it is not a palindrome.
#   4. Advance left, retreat right, repeat until pointers meet.
#
# Entry:  a0 = pointer to null-terminated string
# Return: a0 = 1 if palindrome, 0 if not

        .data
test1:  .string "racecar"              # palindrome
test2:  .string "hello"                # not a palindrome
test3:  .string "a"                    # palindrome (single char)
test4:  .string "abba"                 # palindrome
test5:  .string ""                     # empty string — palindrome by convention

        .text
        .globl main

# ── is_palindrome ────────────────────────────────────────────────
# a0 = pointer to string → a0 = 1 (yes) or 0 (no)

is_palindrome:
        addi    sp, sp, -16
        sw      ra, 12(sp)
        sw      s0,  8(sp)
        sw      s1,  4(sp)
        sw      s2,  0(sp)

        mv      s0, a0                 # s0 = string base pointer

        # Step 1: find length
        # Walk to null terminator
        mv      t0, a0                 # t0 = current pointer
        li      t1, 0                   # t1 = length

find_len:
        lb      t2, 0(t0)             # load byte
        beqz    t2, len_found          # if null, stop
        addi    t1, t1, 1             # length++
        addi    t0, t0, 1             # advance pointer
        j       find_len

len_found:
        # t1 = length
        mv      s1, t1                 # s1 = length

        # Step 2: set up two pointers
        mv      s2, s0                 # s2 = left pointer = base
        add     t0, s0, s1            # t0 = base + length
        addi    t0, t0, -1            # t0 = right pointer = last char

        # Step 3: compare
        ble     s1, zero, is_pal_yes   # length <= 1 → palindrome

cmp_loop:
        bge     s2, t0, is_pal_yes     # if left >= right, all matched

        lb      t1, 0(s2)             # t1 = char at left
        lb      t2, 0(t0)             # t2 = char at right
        bne     t1, t2, is_pal_no      # if different → not a palindrome

        addi    s2, s2, 1             # left++
        addi    t0, t0, -1            # right--
        j       cmp_loop

is_pal_yes:
        li      a0, 1                  # return 1 (true)
        j       pal_ret

is_pal_no:
        li      a0, 0                  # return 0 (false)

pal_ret:
        lw      ra, 12(sp)
        lw      s0,  8(sp)
        lw      s1,  4(sp)
        lw      s2,  0(sp)
        addi    sp, sp, 16
        ret

# ── main — test with several strings ─────────────────────────────
main:
        la      a0, test1
        jal     ra, is_palindrome       # "racecar" → a0 = 1

        la      a0, test2
        jal     ra, is_palindrome       # "hello" → a0 = 0

        la      a0, test3
        jal     ra, is_palindrome       # "a" → a0 = 1

        la      a0, test4
        jal     ra, is_palindrome       # "abba" → a0 = 1

        la      a0, test5
        jal     ra, is_palindrome       # "" → a0 = 1

        li      a7, 10
        ecall
