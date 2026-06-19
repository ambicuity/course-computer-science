/* main.c — branches, loops, recursion, plus binary search with explicit invariant.
 *
 * Build: gcc -O0 -g main.c -o main
 * Run:   ./main
 */

#include <stdio.h>
#include <assert.h>

static const char* classify_if(int x) {
    if (x < 0) return "negative";
    else if (x == 0) return "zero";
    else return "positive";
}

static const char* classify_switch(int x) {
    /* switch can't directly compare ranges; encode as a small key */
    int key = (x < 0) ? -1 : (x == 0 ? 0 : 1);
    switch (key) {
        case -1: return "negative";
        case 0:  return "zero";
        case 1:  return "positive";
        default: return "?";
    }
}

static int sum_for(int n) {
    int s = 0;
    for (int i = 1; i <= n; ++i) s += i;
    return s;
}

static int sum_while(int n) {
    int s = 0, i = 1;
    while (i <= n) { s += i; ++i; }
    return s;
}

static int sum_do_while(int n) {
    int s = 0, i = 1;
    if (n < 1) return 0;       /* do-while always runs once; guard */
    do { s += i; ++i; } while (i <= n);
    return s;
}

/* Binary search.
 * Invariant (at top of every iteration): if target is in arr at all,
 *   then it is in arr[lo .. hi).
 * Initialization: lo=0, hi=n  →  arr[0..n) is the whole array.
 * Maintenance: at each step we halve the search interval.
 * Termination: when lo == hi, the interval is empty → target not found.
 */
static int binary_search(const int *arr, int n, int target) {
    int lo = 0, hi = n;
    while (lo < hi) {
        int mid = lo + (hi - lo) / 2;
        if (arr[mid] == target)      return mid;
        else if (arr[mid] < target)  lo = mid + 1;
        else                          hi = mid;
    }
    return -1;
}

static long long factorial_rec(int n) {
    if (n <= 1) return 1;
    return (long long)n * factorial_rec(n - 1);
}

static long long factorial_tail(int n, long long acc) {
    if (n <= 1) return acc;
    return factorial_tail(n - 1, (long long)n * acc);   /* tail call */
}

static long long fib_iter(int n) {
    if (n < 2) return n;
    long long a = 0, b = 1;
    for (int i = 2; i <= n; ++i) {
        long long c = a + b;
        a = b;
        b = c;
    }
    return b;
}

int main(void) {
    printf("== Branches ==\n");
    for (int x = -3; x <= 3; ++x) {
        printf("  classify(%d) = %s (if), %s (switch)\n", x, classify_if(x), classify_switch(x));
    }

    printf("\n== Three loop forms (all should give 55) ==\n");
    printf("  sum 1..10 for=%d while=%d do-while=%d\n",
           sum_for(10), sum_while(10), sum_do_while(10));
    assert(sum_for(10) == 55 && sum_while(10) == 55 && sum_do_while(10) == 55);

    printf("\n== Binary search ==\n");
    int arr[] = {1, 3, 5, 7, 9, 11, 13, 15, 17, 19};
    int n = sizeof(arr) / sizeof(*arr);
    int targets[] = {1, 7, 19, 20, 0};
    for (int t = 0; t < (int)(sizeof(targets) / sizeof(*targets)); ++t) {
        int idx = binary_search(arr, n, targets[t]);
        printf("  binary_search(arr, %d) = %d\n", targets[t], idx);
    }

    printf("\n== Factorial: recursive vs tail-recursive ==\n");
    for (int i = 0; i <= 12; ++i) {
        long long a = factorial_rec(i);
        long long b = factorial_tail(i, 1);
        printf("  %d! = %lld (rec) = %lld (tail)  %s\n",
               i, a, b, (a == b) ? "✓" : "✗");
        assert(a == b);
    }

    printf("\n== Fibonacci (iterative) ==\n");
    for (int i = 0; i <= 12; ++i) {
        printf("  F_%d = %lld\n", i, fib_iter(i));
    }
    assert(fib_iter(10) == 55);

    return 0;
}
