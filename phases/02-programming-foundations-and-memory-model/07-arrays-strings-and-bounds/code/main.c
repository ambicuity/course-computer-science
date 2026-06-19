/* main.c — array decay, C strings, snprintf, safe copy. */
#include <stdio.h>
#include <string.h>

static void show_inside(int arr[10]) {
    /* Despite the [10] in the declaration, arr is just a pointer here.
     * sizeof(arr) is sizeof(void*), not 10 * sizeof(int). */
    printf("  inside  f: sizeof(arr) = %zu\n", sizeof(arr));
}

static size_t safe_strcpy(char *dst, size_t dst_size, const char *src) {
    if (dst_size == 0) return 0;
    size_t i = 0;
    for (; i < dst_size - 1 && src[i] != '\0'; ++i) {
        dst[i] = src[i];
    }
    dst[i] = '\0';
    return i;
}

int main(int argc, char **argv) {
    printf("== Array decay ==\n");
    int arr[10] = {0};
    printf("  outside f: sizeof(arr) = %zu  (= 10 * sizeof(int))\n", sizeof(arr));
    show_inside(arr);

    printf("\n== strlen vs sizeof ==\n");
    char buf[5] = "abcd";   /* 4 chars + '\0' fits in 5 */
    printf("  buf = \"%s\"\n", buf);
    printf("  strlen(buf) = %zu  (chars before null)\n", strlen(buf));
    printf("  sizeof(buf) = %zu  (buffer capacity including null)\n", sizeof(buf));

    printf("\n== snprintf — safe formatted print ==\n");
    char small[8];
    int written = snprintf(small, sizeof(small), "%s", "much longer string");
    printf("  snprintf 'much longer string' into 8-byte buffer → \"%s\"\n", small);
    printf("  return value: %d  (would-be length; > buffer size means truncated)\n", written);
    printf("  truncation: %s\n", (written >= (int)sizeof(small)) ? "yes" : "no");

    printf("\n== safe_strcpy ==\n");
    char dst[8];
    size_t n = safe_strcpy(dst, sizeof(dst), "much longer string");
    printf("  copied %zu chars: \"%s\"   (always null-terminated)\n", n, dst);

    if (argc > 1 && strcmp(argv[1], "--strcpy-overflow") == 0) {
        printf("\n== Deliberate strcpy overflow (ASan should catch) ==\n");
        char tiny[8];
        const char *long_str = "this string is way longer than 8 bytes";
        strcpy(tiny, long_str);   /* UB */
        printf("  tiny = %s\n", tiny);
    }

    return 0;
}
