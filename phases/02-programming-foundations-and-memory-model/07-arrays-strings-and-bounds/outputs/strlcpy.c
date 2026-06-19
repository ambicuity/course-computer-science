/* strlcpy.c — portable strlcpy/strlcat (OpenBSD-style safe string copy).
 *
 * Build:  gcc -DSTRLCPY_DEMO strlcpy.c -o strlcpy_demo
 * Run:    ./strlcpy_demo
 *
 * Contract:
 *   strlcpy(dst, src, dst_size) copies up to dst_size - 1 chars, always
 *   null-terminates (if dst_size > 0), and returns strlen(src) — so the
 *   caller can detect truncation (returned > dst_size - 1 means truncated).
 */

#include <stddef.h>
#include <string.h>

size_t my_strlcpy(char *dst, const char *src, size_t dst_size) {
    size_t src_len = 0;
    while (src[src_len] != '\0') src_len++;

    if (dst_size > 0) {
        size_t to_copy = src_len;
        if (to_copy >= dst_size) to_copy = dst_size - 1;
        for (size_t i = 0; i < to_copy; i++) dst[i] = src[i];
        dst[to_copy] = '\0';
    }
    return src_len;        /* would-be length */
}

size_t my_strlcat(char *dst, const char *src, size_t dst_size) {
    size_t dst_len = 0;
    while (dst_len < dst_size && dst[dst_len] != '\0') dst_len++;

    size_t src_len = 0;
    while (src[src_len] != '\0') src_len++;

    if (dst_len == dst_size) return dst_size + src_len;  /* dst not null-term */

    size_t avail = dst_size - dst_len - 1;
    size_t to_copy = src_len < avail ? src_len : avail;
    for (size_t i = 0; i < to_copy; i++) dst[dst_len + i] = src[i];
    dst[dst_len + to_copy] = '\0';
    return dst_len + src_len;
}

#ifdef STRLCPY_DEMO
#include <stdio.h>
#include <assert.h>

int main(void) {
    char dst[8];
    size_t n;

    n = my_strlcpy(dst, "hi", sizeof(dst));
    printf("my_strlcpy('hi', 8) → \"%s\"  (returned %zu)\n", dst, n);
    assert(n == 2 && strcmp(dst, "hi") == 0);

    n = my_strlcpy(dst, "way too long", sizeof(dst));
    printf("my_strlcpy('way too long', 8) → \"%s\"  (returned %zu = would-be len; truncated? %s)\n",
           dst, n, n >= sizeof(dst) ? "yes" : "no");
    assert(n == strlen("way too long"));      /* 12 */
    assert(strcmp(dst, "way too") == 0);

    char buf[16] = "abc";
    n = my_strlcat(buf, "defg", sizeof(buf));
    printf("strlcat 'abc' + 'defg' → \"%s\"  (returned %zu)\n", buf, n);
    assert(strcmp(buf, "abcdefg") == 0);
    return 0;
}
#endif
