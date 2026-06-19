.section .text
.globl _start
_start:
  mov $1, %rax
  mov $1, %rdi
  lea msg(%rip), %rsi
  mov $17, %rdx
  syscall

  mov $60, %rax
  xor %rdi, %rdi
  syscall

.section .rodata
msg: .ascii "syscall boundary\n"
