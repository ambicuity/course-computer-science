/* ptr_safety.h — tiny header full of pointer-safety macros.
 *
 * Drop into any C project. Pair with `-fsanitize=address,undefined` at -O0/-O1.
 */
#ifndef PTR_SAFETY_H
#define PTR_SAFETY_H

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

/* Free p and NULL it in one go — eliminates double-free and use-after-free in the common case. */
#define SAFE_FREE(p) do {              \
        free(p);                       \
        (p) = NULL;                    \
    } while (0)

/* Abort if a malloc returned NULL. Use only for cases where recovery is impossible. */
#define XMALLOC(size) ({                                                      \
        void *_p = malloc(size);                                              \
        if (!_p) { fprintf(stderr, "xmalloc(%zu) failed\n", (size_t)(size)); \
                   abort(); }                                                  \
        _p;                                                                    \
    })

/* Bounds-check before indexing. Abort with a clear message instead of corrupting memory. */
#define BOUNDS_CHECK(arr, i, len) do {                                          \
        if ((size_t)(i) >= (size_t)(len)) {                                     \
            fprintf(stderr, "%s:%d: bounds violation: index %zu >= len %zu\n",  \
                    __FILE__, __LINE__, (size_t)(i), (size_t)(len));            \
            abort();                                                            \
        }                                                                       \
    } while (0)

/* Read an element with bounds checking. Returns the value. */
#define ARR_AT(arr, i, len) (BOUNDS_CHECK((arr), (i), (len)), (arr)[(i)])

/* Cast through memcpy to dodge strict-aliasing UB. */
#define BIT_CAST(DST_TYPE, SRC) ({                          \
        DST_TYPE _dst;                                       \
        _Static_assert(sizeof(_dst) == sizeof(SRC),          \
                       "BIT_CAST requires same-size types"); \
        memcpy(&_dst, &(SRC), sizeof(_dst));                 \
        _dst;                                                \
    })

#endif /* PTR_SAFETY_H */
