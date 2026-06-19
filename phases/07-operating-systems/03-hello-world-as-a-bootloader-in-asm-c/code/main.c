#include <stdio.h>

int main(void) {
  puts("hello from freestanding-style C stub");
  puts("bootloaders normally avoid libc and write directly to memory-mapped IO");
  return 0;
}
