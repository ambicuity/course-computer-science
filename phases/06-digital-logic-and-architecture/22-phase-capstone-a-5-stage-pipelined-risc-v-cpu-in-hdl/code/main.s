.section .text
.globl _start
_start:
  li x1, 1
  li x2, 2
  add x3, x1, x2
  sw x3, 0(x0)
  j _start
