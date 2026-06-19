/* c_errors.h — small helpers for the C goto-cleanup pattern and errno preservation.
 *
 * Idiom:
 *   FILE *fp = NULL;
 *   void *buf = NULL;
 *
 *   fp = fopen(...);  CHECK(fp != NULL, "fopen failed", cleanup_none);
 *   buf = malloc(...); CHECK(buf != NULL, "alloc failed", cleanup_fp);
 *   ...
 *
 *   cleanup_buf: free(buf);
 *   cleanup_fp:  fclose(fp);
 *   cleanup_none:
 *       return rc;
 */
#ifndef C_ERRORS_H
#define C_ERRORS_H

#include <errno.h>
#include <stdio.h>
#include <string.h>

#define CHECK(cond, msg, label) do {                                              \
        if (!(cond)) {                                                            \
            fprintf(stderr, "%s:%d: %s: %s\n",                                    \
                    __FILE__, __LINE__, (msg), strerror(errno));                  \
            goto label;                                                            \
        }                                                                          \
    } while (0)

/* Preserve errno across an unrelated call (e.g., logging that overwrites errno). */
#define PRESERVING_ERRNO(stmt) do {                                                \
        int _saved_errno = errno;                                                  \
        do { stmt; } while (0);                                                    \
        errno = _saved_errno;                                                      \
    } while (0)

/* Simple status enum for libraries that want named return codes. */
typedef enum {
    OK         = 0,
    E_INVAL    = 1,    /* invalid argument */
    E_NOMEM    = 2,    /* out of memory */
    E_IO       = 3,    /* I/O error */
    E_NOTFOUND = 4,
} status_t;

static inline const char *status_str(status_t s) {
    switch (s) {
        case OK:         return "OK";
        case E_INVAL:    return "invalid argument";
        case E_NOMEM:    return "out of memory";
        case E_IO:       return "I/O error";
        case E_NOTFOUND: return "not found";
        default:         return "unknown";
    }
}

#endif
