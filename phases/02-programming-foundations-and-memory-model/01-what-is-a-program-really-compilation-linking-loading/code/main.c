/* main.c — one example per ELF section, plus a runtime trip through the layout.
 *
 * Build:  gcc -O0 -g main.c -o main
 * Run:    ./main
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* .data (initialized writable global) */
int initialized_global = 42;

/* .bss (uninitialized writable global; zero at load) */
int uninitialized_global;

/* .rodata (const string literal) */
const char *const message = "hello, world";

/* .text — function bodies */
static int increment_static_local(void) {
    static int count = 0;   /* .data (or .bss if 0-init); lifetime = program */
    count += 1;
    return count;
}

int main(int argc, char **argv) {
    (void)argc; (void)argv;

    int stack_var = 7;                 /* on the stack */
    int *heap_var = malloc(sizeof(int));
    if (!heap_var) return 1;
    *heap_var = 99;                    /* on the heap */

    printf("%s\n", message);
    printf("initialized_global    @ %p  = %d\n",
           (void*)&initialized_global, initialized_global);
    printf("uninitialized_global  @ %p  = %d  (zero by default)\n",
           (void*)&uninitialized_global, uninitialized_global);
    printf("message (rodata)      @ %p  = \"%s\"\n",
           (void*)message, message);
    printf("stack_var             @ %p  = %d  (on the stack)\n",
           (void*)&stack_var, stack_var);
    printf("heap_var               points to %p  = %d  (on the heap)\n",
           (void*)heap_var, *heap_var);

    for (int i = 0; i < 3; ++i) {
        printf("increment_static_local() = %d  (static keeps state across calls)\n",
               increment_static_local());
    }

    free(heap_var);
    return 0;
}
