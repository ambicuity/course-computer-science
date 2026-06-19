.section .text
.globl add_mul
add_mul:
  # int add_mul(int a, int b, int c) => (a + b) * c
  mov %edi, %eax
  add %esi, %eax
  imul %edx, %eax
  ret
