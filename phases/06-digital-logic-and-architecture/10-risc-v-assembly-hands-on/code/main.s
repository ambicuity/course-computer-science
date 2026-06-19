.section .text
.globl _start
_start:
  li a0, 0
  li a1, 10
loop:
  add a0, a0, a1
  addi a1, a1, -1
  bnez a1, loop
