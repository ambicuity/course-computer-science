/*
 * suffix_array.h — single-header suffix array + Kasai LCP.
 *
 *   int n = strlen(s);
 *   int *sa = sa_build(s, n);
 *   int *lcp = sa_lcp(s, n, sa);
 *   int pos = sa_search(s, n, sa, "pattern");
 *   free(sa); free(lcp);
 *
 * Construction: O(n log^2 n) via doubling sort. For O(n) use SA-IS.
 * LCP: O(n) via Kasai's algorithm.
 */
#ifndef SUFFIX_ARRAY_H
#define SUFFIX_ARRAY_H

#include <stdlib.h>
#include <string.h>

static int *sa__rank, *sa__newrank;
static int  sa__N, sa__K;
static const char *sa__T;

static int sa__cmp(const void *a, const void *b) {
    int i = *(const int *)a, j = *(const int *)b;
    if (sa__rank[i] != sa__rank[j]) return sa__rank[i] - sa__rank[j];
    int ri = (i + sa__K < sa__N) ? sa__rank[i + sa__K] : -1;
    int rj = (j + sa__K < sa__N) ? sa__rank[j + sa__K] : -1;
    return ri - rj;
}

static inline int *sa_build(const char *t, int n) {
    sa__T = t; sa__N = n;
    int *sa = (int *)malloc(n * sizeof(int));
    sa__rank = (int *)malloc(n * sizeof(int));
    sa__newrank = (int *)malloc(n * sizeof(int));
    for (int i = 0; i < n; ++i) { sa[i] = i; sa__rank[i] = (unsigned char)t[i]; }
    for (int k = 1; ; k *= 2) {
        sa__K = k;
        qsort(sa, n, sizeof(int), sa__cmp);
        sa__newrank[sa[0]] = 0;
        for (int i = 1; i < n; ++i) {
            sa__newrank[sa[i]] = sa__newrank[sa[i - 1]];
            if (sa__cmp(&sa[i - 1], &sa[i]) != 0) sa__newrank[sa[i]]++;
        }
        memcpy(sa__rank, sa__newrank, n * sizeof(int));
        if (sa__rank[sa[n - 1]] == n - 1) break;
    }
    free(sa__rank); free(sa__newrank);
    sa__rank = NULL; sa__newrank = NULL;
    return sa;
}

static inline int *sa_lcp(const char *t, int n, const int *sa) {
    int *isa = (int *)malloc(n * sizeof(int));
    int *lcp = (int *)calloc(n, sizeof(int));
    for (int i = 0; i < n; ++i) isa[sa[i]] = i;
    int h = 0;
    for (int i = 0; i < n; ++i) {
        if (isa[i] > 0) {
            int j = sa[isa[i] - 1];
            while (i + h < n && j + h < n && t[i + h] == t[j + h]) ++h;
            lcp[isa[i]] = h;
            if (h > 0) --h;
        } else h = 0;
    }
    free(isa);
    return lcp;
}

static inline int sa_search(const char *t, int n, const int *sa, const char *p) {
    int m = (int)strlen(p);
    int lo = 0, hi = n;
    while (lo < hi) {
        int mid = (lo + hi) / 2;
        int c = strncmp(t + sa[mid], p, m);
        if (c < 0) lo = mid + 1; else hi = mid;
    }
    if (lo < n && strncmp(t + sa[lo], p, m) == 0) return sa[lo];
    return -1;
}

#endif /* SUFFIX_ARRAY_H */
