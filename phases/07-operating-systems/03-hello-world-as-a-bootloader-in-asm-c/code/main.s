.section .text
.globl _start
_start:
  xor %eax, %eax
  mov $60, %eax
  xor %edi, %edi
  syscall
