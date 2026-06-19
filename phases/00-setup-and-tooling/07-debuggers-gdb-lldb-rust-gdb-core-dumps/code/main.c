/* Debugger drill program.
 *
 * Build:  gcc -g -O0 main.c -o main
 * Run:
 *   ./main 5    → normal path; prints sum of 1..5
 *   ./main 999  → triggers abort()  (for the core-dump exercise)
 *
 * The function `compute_sum` has a deliberate off-by-one: it writes one past
 * the end of `arr` (read the source AFTER you've found it in gdb).
 */

#include <stdio.h>
#include <stdlib.h>
#include <assert.h>

static int g_state = 0;

static int compute_sum(int n) {
    /* Buffer large enough to hold the bug without overrunning the stack. */
    int arr[8] = {0};        /* zero-init defeats the canary; the bug below is now a logic bug */
    if (n > 7) {
        fprintf(stderr, "n=%d too large (this build's arr is 7 slots)\n", n);
        return -1;
    }
    int sum = 0;
    /* BUG: writes arr[i] for i in 1..n; arr[0] is never touched on this path.
     * Find it in gdb by watching arr[0] (it never changes) and noticing g_state
     * comes out wrong as a result.
     */
    for (int i = 1; i <= n; ++i) {
        arr[i] = i;          /* should be arr[i-1] */
        sum += i;
    }
    g_state = arr[0] + arr[1] + (n >= 2 ? arr[2] : 0);   /* arr[0] is always 0 → wrong total */
    return sum;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s N (1..5)\n", argv[0]);
        return 1;
    }
    int n = atoi(argv[1]);
    if (n == 999) {
        fprintf(stderr, "deliberate abort: triggering SIGABRT for core dump demo\n");
        abort();
    }
    int s = compute_sum(n);
    printf("sum(1..%d) = %d  (g_state=%d)\n", n, s, g_state);
    return 0;
}
