/* main.c — read kernel state from /proc using plain file I/O.
 *
 * Works on Linux. macOS does not have /proc; the program prints a notice and exits.
 *
 * Build:  gcc main.c -o procreader
 * Run:    ./procreader
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <sys/stat.h>

static int file_exists(const char *path) {
    struct stat st;
    return stat(path, &st) == 0;
}

static void print_file(const char *path, int max_lines) {
    FILE *f = fopen(path, "r");
    if (!f) {
        printf("  (cannot open %s: %s)\n", path, strerror(errno));
        return;
    }
    char line[512];
    int n = 0;
    while (fgets(line, sizeof(line), f) && n < max_lines) {
        printf("  %s", line);
        size_t len = strlen(line);
        if (len > 0 && line[len-1] != '\n') printf("\n");
        ++n;
    }
    fclose(f);
}

int main(void) {
    if (!file_exists("/proc/self/status")) {
        printf("This program reads /proc, which exists on Linux.\n");
        printf("On macOS, equivalent info comes from sysctl / Mach APIs.\n");
        return 0;
    }

    printf("── /proc/self/status (first 12 lines) ───────────────\n");
    print_file("/proc/self/status", 12);

    printf("\n── /proc/uptime (seconds since boot, idle CPU-seconds) ──\n");
    print_file("/proc/uptime", 1);

    printf("\n── /proc/loadavg (1, 5, 15-min loads + runnable/total + last PID) ──\n");
    print_file("/proc/loadavg", 1);

    printf("\n── /proc/meminfo (first 6 lines) ────────────────────\n");
    print_file("/proc/meminfo", 6);

    printf("\n── /proc/self/cmdline ───────────────────────────────\n");
    FILE *f = fopen("/proc/self/cmdline", "r");
    if (f) {
        int c;
        printf("  ");
        while ((c = fgetc(f)) != EOF) putchar(c == 0 ? ' ' : c);
        putchar('\n');
        fclose(f);
    }

    return 0;
}
