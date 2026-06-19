/* main.c — assert + ASAN/UBSAN demos. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

static int sum(const int *arr, size_t n) {
    assert(arr != NULL && "sum: arr must not be NULL");
    assert(n > 0       && "sum: n must be > 0");
    int s = 0;
    for (size_t i = 0; i < n; ++i) s += arr[i];
    return s;
}

static int is_sorted(const int *arr, size_t n) {
    for (size_t i = 1; i < n; ++i) {
        if (arr[i - 1] > arr[i]) return 0;
    }
    return 1;
}

static int bsearch_int(const int *arr, size_t n, int target) {
    assert(is_sorted(arr, n) && "bsearch: array must be sorted");
    size_t lo = 0, hi = n;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (arr[mid] == target) return (int)mid;
        if (arr[mid] < target) lo = mid + 1;
        else                    hi = mid;
    }
    return -1;
}

int main(int argc, char **argv) {
    printf("== Asserts hold the happy path silently ==\n");
    int arr[] = {1, 3, 5, 7, 9};
    int s = sum(arr, 5);
    printf("  sum([1,3,5,7,9]) = %d\n", s);
    int idx = bsearch_int(arr, 5, 7);
    printf("  bsearch_int(arr, 7) = %d\n", idx);

    if (argc > 1 && strcmp(argv[1], "--null") == 0) {
        printf("\n== Triggering null assert ==\n");
        sum(NULL, 5);   /* assert fires; process aborts */
    }
    if (argc > 1 && strcmp(argv[1], "--unsorted") == 0) {
        printf("\n== Triggering 'array not sorted' assert ==\n");
        int bad[] = {3, 1, 2};
        bsearch_int(bad, 3, 1);   /* assert fires */
    }
    if (argc > 1 && strcmp(argv[1], "--asan-oob") == 0) {
        printf("\n== ASAN heap-buffer-overflow demo ==\n");
        int *p = malloc(4 * sizeof(int));
        p[10] = 42;
        free(p);
    }
    if (argc > 1 && strcmp(argv[1], "--leak") == 0) {
        printf("\n== LSAN leak demo ==\n");
        (void)malloc(1024);
    }

    return 0;
}
