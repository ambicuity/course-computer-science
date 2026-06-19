/* main.c — C error-handling models: return codes, errno, goto-cleanup. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <fcntl.h>
#include <unistd.h>

/* Pattern 1: return code + errno */
static int demo_errno(void) {
    FILE *fp = fopen("/does/not/exist", "r");
    if (!fp) {
        /* errno is set; strerror prints the human message */
        printf("  fopen failed: errno=%d (%s)\n", errno, strerror(errno));
        return 1;
    }
    fclose(fp);
    return 0;
}

/* Pattern 2: goto cleanup — the canonical C way to avoid leaks on error.
 * Each successful step has a corresponding cleanup label. */
static int load_and_process(const char *path, int *out) {
    int rc = -1;
    FILE *fp = NULL;
    char *buf = NULL;

    fp = fopen(path, "r");
    if (!fp) goto cleanup_none;

    buf = malloc(1024);
    if (!buf) goto cleanup_fp;

    if (!fgets(buf, 1024, fp)) goto cleanup_all;

    *out = atoi(buf);
    rc = 0;

cleanup_all:
    free(buf);
cleanup_fp:
    fclose(fp);
cleanup_none:
    if (rc != 0) printf("  load_and_process(%s) failed at one of the steps\n", path);
    return rc;
}

int main(void) {
    printf("== Pattern 1: errno (POSIX-style) ==\n");
    demo_errno();

    printf("\n== Pattern 2: goto cleanup chain ==\n");
    /* Create a file with a number, parse it via load_and_process */
    const char *tmp = "/tmp/lesson_demo_number.txt";
    FILE *w = fopen(tmp, "w");
    if (w) { fputs("42\n", w); fclose(w); }

    int v = 0;
    int rc = load_and_process(tmp, &v);
    printf("  load_and_process(%s) → rc=%d, parsed value=%d\n", tmp, rc, v);
    remove(tmp);

    rc = load_and_process("/does/not/exist", &v);
    printf("  load_and_process(\"/does/not/exist\") → rc=%d   (cleanup chain handled failure)\n", rc);
    return 0;
}
