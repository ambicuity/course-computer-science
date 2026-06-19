/* main.c — exercise function calls, the stack, and recursion. */
#include <stdio.h>

static long sum6(long a, long b, long c, long d, long e, long f) {
    return a + b + c + d + e + f;
}

static long sum8(long a, long b, long c, long d, long e, long f, long g, long h) {
    /* On SysV AMD64, args g and h come from the stack (only 6 register slots). */
    return a + b + c + d + e + f + g + h;
}

static long factorial(long n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

static void print_stack_addresses(int depth) {
    int local;
    printf("  depth %2d: stack local at %p\n", depth, (void*)&local);
    if (depth < 5) print_stack_addresses(depth + 1);
}

int main(void) {
    printf("== Stack growth direction (each call's local at a LOWER address) ==\n");
    print_stack_addresses(0);

    printf("\n== sum6(1..6) = %ld   (all 6 args in registers)\n", sum6(1, 2, 3, 4, 5, 6));
    printf("== sum8(1..8) = %ld   (args 7, 8 passed on the stack)\n",
           sum8(1, 2, 3, 4, 5, 6, 7, 8));

    printf("\n== factorial(10) = %ld  (10 recursive frames)\n", factorial(10));
    return 0;
}
