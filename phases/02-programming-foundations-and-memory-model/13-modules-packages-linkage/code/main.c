/* main.c — single-file demo of forward declarations + static (internal linkage).
 *
 * Build:  gcc main.c -o main
 * Run:    ./main
 *
 * The full multi-file project (with a Makefile) lives in
 * outputs/c-project-skeleton/.
 */
#include <stdio.h>

/* Forward declarations — what a header would normally provide. */
int add(int a, int b);
int sub(int a, int b);
int read_counter(void);

int main(void) {
    printf("add(3, 4) = %d\n", add(3, 4));
    printf("sub(10, 7) = %d\n", sub(10, 7));
    printf("calls so far = %d\n", read_counter());
    return 0;
}

/* counter has INTERNAL linkage — accessible only within this translation unit. */
static int counter = 0;

int add(int a, int b) { counter++; return a + b; }
int sub(int a, int b) { counter++; return a - b; }
int read_counter(void) { return counter; }
