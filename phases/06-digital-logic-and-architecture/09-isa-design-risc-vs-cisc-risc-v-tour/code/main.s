.section .text
.globl _start
_start:
  li t0, 5
  li t1, 7
  add t2, t0, t1
  sub t3, t2, t0
  and t4, t2, t3
  or  t5, t2, t3
