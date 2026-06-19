/* binary_search.c — textbook implementation with explicit loop invariant
 * and property-based-style fuzz tests.
 *
 * Build:  gcc binary_search.c -o bs
 * Run:    ./bs
 *
 * Used downstream by Phase 04 (search algorithms) and as a sanity reference.
 */

#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include <time.h>

/*
 * Loop invariant (held at top of every iteration):
 *
 *   if target appears in arr at all, then it is in arr[lo..hi) — the
 *   half-open interval from index lo (inclusive) to hi (exclusive).
 *
 * Initialization: lo=0, hi=n covers the whole array.
 * Maintenance:    each iteration halves the interval, preserving the invariant.
 * Termination:    when lo == hi, the interval is empty → target not present.
 */
int binary_search(const int *arr, int n, int target) {
    int lo = 0, hi = n;
    while (lo < hi) {
        int mid = lo + (hi - lo) / 2;        /* overflow-safe midpoint */
        if (arr[mid] == target)      return mid;
        else if (arr[mid] < target)  lo = mid + 1;
        else                          hi = mid;
    }
    return -1;
}

/* Sorted-array property-style test: insert values 0..n-1, search for each. */
static void test_property(int n) {
    int *arr = malloc(sizeof(int) * n);
    for (int i = 0; i < n; ++i) arr[i] = i * 2;     /* sorted, distinct */
    for (int i = 0; i < n; ++i) {
        int found = binary_search(arr, n, i * 2);
        assert(found == i);
    }
    /* odd values aren't present */
    for (int i = 0; i < n; ++i) {
        assert(binary_search(arr, n, i * 2 + 1) == -1);
    }
    free(arr);
}

int main(void) {
    /* Spot checks */
    int sample[] = {1, 3, 5, 7, 9, 11, 13, 15};
    assert(binary_search(sample, 8, 1)  == 0);
    assert(binary_search(sample, 8, 7)  == 3);
    assert(binary_search(sample, 8, 15) == 7);
    assert(binary_search(sample, 8, 4)  == -1);

    /* Property tests */
    int sizes[] = {1, 2, 3, 7, 100, 1000, 10000};
    for (size_t i = 0; i < sizeof(sizes) / sizeof(*sizes); ++i) {
        test_property(sizes[i]);
    }
    printf("binary_search: property tests passed for n ∈ {1, 2, 3, 7, 100, 1000, 10000}\n");
    return 0;
}
