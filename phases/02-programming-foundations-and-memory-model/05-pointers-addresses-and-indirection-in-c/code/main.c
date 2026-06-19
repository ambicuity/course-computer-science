/* main.c — pointer fundamentals + a bug switch for ASan demos. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int add(int a, int b) { return a + b; }
static int mul(int a, int b) { return a * b; }

static int combine(int x, int y, int (*op)(int, int)) {
    return op(x, y);
}

int main(int argc, char **argv) {
    int demo_oob = (argc > 1 && strcmp(argv[1], "--oob") == 0);

    printf("== Address-of and dereference ==\n");
    int x = 42;
    int *p = &x;
    printf("  x = %d,  &x = %p,  *p = %d  (p == &x ? %s)\n",
           x, (void*)&x, *p, (p == &x ? "yes" : "no"));

    printf("\n== Pointer arithmetic stride differs by element type ==\n");
    int arr[4] = {10, 20, 30, 40};
    int  *pi = arr;
    char *pc = (char *)arr;
    printf("  int  *pi:  pi=%p, pi+1=%p, diff = %ld bytes (= sizeof(int))\n",
           (void*)pi, (void*)(pi + 1), (long)((char*)(pi + 1) - (char*)pi));
    printf("  char *pc:  pc=%p, pc+1=%p, diff = %ld byte  (= sizeof(char))\n",
           (void*)pc, (void*)(pc + 1), (long)((char*)(pc + 1) - (char*)pc));

    printf("\n== Subscript ≡ *(p + i) ==\n");
    printf("  arr[2] = %d,  *(arr + 2) = %d,  2[arr] = %d\n",
           arr[2], *(arr + 2), 2[arr]);

    printf("\n== Function pointers ==\n");
    int (*fn)(int, int) = add;
    printf("  fn = add: combine(3, 4, fn) = %d\n", combine(3, 4, fn));
    fn = mul;
    printf("  fn = mul: combine(3, 4, fn) = %d\n", combine(3, 4, fn));

    printf("\n== void * and memcpy-based type punning ==\n");
    int   from = 0x41424344;       /* 'ABCD' as little-endian bytes */
    char  bytes[4];
    memcpy(bytes, &from, sizeof(from));
    printf("  int 0x%08x as bytes: %02x %02x %02x %02x  (= '%c%c%c%c' on little-endian)\n",
           from, (unsigned char)bytes[0], (unsigned char)bytes[1],
           (unsigned char)bytes[2], (unsigned char)bytes[3],
           bytes[0], bytes[1], bytes[2], bytes[3]);

    if (demo_oob) {
        printf("\n== Deliberate OOB read (compile with -fsanitize=address to see ASan catch it) ==\n");
        volatile int *q = arr;
        printf("  arr[4] (past end) = %d   ← UB\n", q[4]);
    }

    return 0;
}
