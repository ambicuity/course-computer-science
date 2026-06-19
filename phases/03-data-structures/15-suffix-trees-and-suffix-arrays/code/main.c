/* main.c — Suffix array (doubling sort) + Kasai's LCP + substring search.
 *
 * SA built in O(n log^2 n). Kasai's LCP in O(n).
 * For O(n) SA construction, see SA-IS (Nong-Zhang-Chan 2009).
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

static const char *T;
static int N;
static int *RANK;          /* current rank per suffix */
static int *NEW_RANK;
static int K_GLOB;         /* current doubling distance */

static int cmp_pairs(const void *a, const void *b) {
    int i = *(const int *)a, j = *(const int *)b;
    if (RANK[i] != RANK[j]) return RANK[i] - RANK[j];
    int ri = (i + K_GLOB < N) ? RANK[i + K_GLOB] : -1;
    int rj = (j + K_GLOB < N) ? RANK[j + K_GLOB] : -1;
    return ri - rj;
}

static int *build_sa(const char *t, int n) {
    T = t; N = n;
    int *sa = malloc(n * sizeof(int));
    RANK = malloc(n * sizeof(int));
    NEW_RANK = malloc(n * sizeof(int));
    for (int i = 0; i < n; ++i) { sa[i] = i; RANK[i] = (unsigned char)t[i]; }

    for (int k = 1; ; k *= 2) {
        K_GLOB = k;
        qsort(sa, n, sizeof(int), cmp_pairs);

        NEW_RANK[sa[0]] = 0;
        for (int i = 1; i < n; ++i) {
            NEW_RANK[sa[i]] = NEW_RANK[sa[i - 1]];
            if (cmp_pairs(&sa[i - 1], &sa[i]) != 0) NEW_RANK[sa[i]]++;
        }
        memcpy(RANK, NEW_RANK, n * sizeof(int));
        if (RANK[sa[n - 1]] == n - 1) break;          /* all distinct → done */
    }
    free(NEW_RANK);
    return sa;                                         /* RANK leaks; OK for demo */
}

/* Kasai's algorithm: O(n) LCP from SA. */
static int *build_lcp(const char *t, int n, const int *sa) {
    int *isa = malloc(n * sizeof(int));
    int *lcp = calloc(n, sizeof(int));
    for (int i = 0; i < n; ++i) isa[sa[i]] = i;
    int h = 0;
    for (int i = 0; i < n; ++i) {
        if (isa[i] > 0) {
            int j = sa[isa[i] - 1];
            while (i + h < n && j + h < n && t[i + h] == t[j + h]) ++h;
            lcp[isa[i]] = h;
            if (h > 0) --h;
        } else {
            h = 0;
        }
    }
    free(isa);
    return lcp;
}

/* Binary search for substring P in T using SA. Returns -1 if not present. */
static int sa_search(const char *t, int n, const int *sa, const char *p) {
    int m = strlen(p);
    int lo = 0, hi = n;
    while (lo < hi) {
        int mid = (lo + hi) / 2;
        int c = strncmp(t + sa[mid], p, m);
        if (c < 0) lo = mid + 1;
        else       hi = mid;
    }
    if (lo < n && strncmp(t + sa[lo], p, m) == 0) return sa[lo];
    return -1;
}

/* Longest repeated substring: max LCP gives length, position from SA. */
static void longest_repeat(const char *t, int n, const int *sa, const int *lcp) {
    int max_lcp = 0, idx = 0;
    for (int i = 1; i < n; ++i) {
        if (lcp[i] > max_lcp) { max_lcp = lcp[i]; idx = sa[i]; }
    }
    printf("  longest repeated substring: length=%d at offset %d → ", max_lcp, idx);
    for (int k = 0; k < max_lcp; ++k) putchar(t[idx + k]);
    putchar('\n');
}

int main(void) {
    const char *text = "the quick brown fox jumps over the lazy dog. the quick fox is quick.";
    int n = strlen(text);

    int *sa = build_sa(text, n);
    int *lcp = build_lcp(text, n, sa);

    printf("== Suffix array + LCP for: \"%s\" ==\n", text);
    printf("  n=%d, first 5 suffixes (sorted):\n", n);
    for (int i = 0; i < 5; ++i) {
        printf("    SA[%d]=%2d LCP=%d  \"", i, sa[i], lcp[i]);
        for (int k = sa[i]; k < n && k < sa[i] + 30; ++k) putchar(text[k]);
        printf("\"\n");
    }

    longest_repeat(text, n, sa, lcp);

    int pos = sa_search(text, n, sa, "quick");
    printf("  search 'quick' → offset %d\n", pos);
    pos = sa_search(text, n, sa, "zebra");
    printf("  search 'zebra' → offset %d (expect -1)\n", pos);

    free(sa); free(lcp);
    return 0;
}
