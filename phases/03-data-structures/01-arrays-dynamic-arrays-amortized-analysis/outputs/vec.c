/*
 * vec.c — dynamic array helpers in C. Type-generic via macros.
 * Drop into your project; uses doubling growth (amortized O(1) push).
 *
 *   VEC_DECL(int);              // declares Vec_int
 *   Vec_int v = {0};
 *   vec_push(int, &v, 42);
 *   for (size_t i = 0; i < v.len; ++i) printf("%d\n", v.data[i]);
 *   vec_free(int, &v);
 */
#ifndef VEC_C
#define VEC_C
#include <stdlib.h>
#include <string.h>
#include <assert.h>

#define VEC_DECL(T) typedef struct { T *data; size_t len, cap; } Vec_##T;

#define vec_reserve(T, v, ncap) do {                                       \
    if ((ncap) > (v)->cap) {                                                \
        (v)->data = (T *)realloc((v)->data, (ncap) * sizeof(T));            \
        assert((v)->data);                                                  \
        (v)->cap = (ncap);                                                  \
    }                                                                       \
} while (0)

#define vec_push(T, v, x) do {                                              \
    if ((v)->len == (v)->cap) {                                             \
        size_t nc = (v)->cap == 0 ? 4 : (v)->cap * 2;                       \
        vec_reserve(T, (v), nc);                                            \
    }                                                                       \
    (v)->data[(v)->len++] = (x);                                            \
} while (0)

#define vec_pop(T, v) ((v)->data[--(v)->len])

#define vec_free(T, v) do { free((v)->data); (v)->data = NULL; (v)->len = (v)->cap = 0; } while (0)

#endif /* VEC_C */
