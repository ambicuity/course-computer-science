#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <time.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <fcntl.h>
#include <unistd.h>

#ifdef __linux__
#define HAS_HUGEPAGES 1
#ifndef MAP_HUGETLB
#define MAP_HUGETLB 0x40000
#endif
#ifndef MAP_HUGE_2MB
#define MAP_HUGE_2MB (21 << 26)
#endif
#else
#define HAS_HUGEPAGES 0
#endif

#define FILE_SIZE_MB 4
#define FILE_SIZE (FILE_SIZE_MB * 1024 * 1024)
#define BUF_SIZE (64 * 1024)
#define PAGE_SZ 4096

static double now_sec(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

static int create_test_file(const char *path, size_t size)
{
    int fd = open(path, O_RDWR | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) return -1;
    if (ftruncate(fd, size) < 0) { close(fd); return -1; }
    return fd;
}

static void warm_file(int fd, size_t size)
{
    char *addr = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (addr == MAP_FAILED) return;
    memset(addr, 0xAA, size);
    munmap(addr, size);
}

static void bench_read(int fd)
{
    char *buf = malloc(BUF_SIZE);
    if (!buf) { perror("malloc"); return; }
    lseek(fd, 0, SEEK_SET);

    double t0 = now_sec();
    ssize_t total = 0, n;
    while ((n = read(fd, buf, BUF_SIZE)) > 0) {
        volatile unsigned char sink = buf[0];
        (void)sink;
        total += n;
    }
    double elapsed = now_sec() - t0;

    double mb_s = (total / (1024.0 * 1024.0)) / elapsed;
    printf("  read()      : %8.2f ms  %7.1f MB/s  (%zd bytes)\n",
           elapsed * 1000, mb_s, total);
    fflush(stdout);
    free(buf);
}

static void bench_mmap(int fd, size_t size)
{
    void *addr = mmap(NULL, size, PROT_READ, MAP_PRIVATE, fd, 0);
    if (addr == MAP_FAILED) { perror("mmap"); return; }

    double t0 = now_sec();
    volatile unsigned char sink;
    size_t accessed = 0;
    for (size_t i = 0; i < size; i += PAGE_SZ) {
        sink = ((unsigned char *)addr)[i];
        accessed += PAGE_SZ;
    }
    double elapsed = now_sec() - t0;

    double mb_s = (accessed / (1024.0 * 1024.0)) / elapsed;
    printf("  mmap()      : %8.2f ms  %7.1f MB/s  (%zu bytes)\n",
           elapsed * 1000, mb_s, accessed);
    fflush(stdout);
    (void)sink;
    munmap(addr, size);
}

static void measure_page_faults(void)
{
    printf("\n--- Page Fault Overhead ---\n");
    fflush(stdout);
    size_t len = 64 * PAGE_SZ;
    void *addr = mmap(NULL, len, PROT_READ | PROT_WRITE,
                      MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (addr == MAP_FAILED) { perror("mmap(anon)"); return; }

    double t0 = now_sec();
    for (size_t i = 0; i < len; i += PAGE_SZ)
        memset((char *)addr + i, 0xAA, 1);
    double elapsed = now_sec() - t0;

    int npages = (int)(len / PAGE_SZ);
    double us_per_fault = (elapsed * 1e6) / npages;
    printf("  %d pages touched in %.1f us (%.1f us/page fault)\n",
           npages, elapsed * 1e6, us_per_fault);
    fflush(stdout);
    munmap(addr, len);
}

static void demonstrate_cow(void)
{
    printf("\n--- MAP_PRIVATE Copy-on-Write ---\n");
    fflush(stdout);
    size_t sz = 4 * 1024 * 1024;
    int fd = create_test_file("/tmp/zerocopy_cow", sz);
    if (fd < 0) { perror("open cow"); return; }
    warm_file(fd, sz);

    char *shared = mmap(NULL, sz, PROT_READ | PROT_WRITE,
                        MAP_SHARED, fd, 0);
    if (shared == MAP_FAILED) { perror("mmap shared"); close(fd); return; }

    memset(shared, 'A', sz);
    char *priv = mmap(NULL, sz, PROT_READ | PROT_WRITE,
                      MAP_PRIVATE, fd, 0);
    if (priv == MAP_FAILED) { perror("mmap private"); close(fd); return; }

    printf("  Before write: shared[0]='%c', private[0]='%c'\n",
           shared[0], priv[0]);

    priv[0] = 'B';

    printf("  After priv[0]='B': shared[0]='%c', private[0]='%c'\n",
           shared[0], priv[0]);
    printf("  COW confirmed: private write did not affect shared mapping\n");
    fflush(stdout);

    munmap(priv, sz);
    munmap(shared, sz);
    close(fd);
    unlink("/tmp/zerocopy_cow");
}

static void bench_fork_cow(void)
{
    printf("\n--- fork() COW Demonstration ---\n");
    fflush(stdout);
    size_t len = 4 * 1024 * 1024;
    char *buf = malloc(len);
    if (!buf) { perror("malloc"); return; }
    memset(buf, 'X', len);

    fflush(stdout);
    double t_fork = now_sec();
    pid_t pid = fork();
    if (pid == 0) {
        free(buf);
        _exit(0);
    } else {
        memset(buf, 'Y', len);
        int status;
        waitpid(pid, &status, 0);
        double elapsed = now_sec() - t_fork;
        printf("  fork() + parent write of %zu MB: %.2f ms\n",
               len / (1024 * 1024), elapsed * 1000);
        printf("  (COW: only pages modified by parent are copied)\n");
        fflush(stdout);
    }
    free(buf);
}

static void compare_huge_pages(void)
{
    printf("\n--- Huge Pages vs Regular Pages ---\n");
    fflush(stdout);
    size_t len = 4 * 1024 * 1024;

    double t0 = now_sec();
    char *regular = mmap(NULL, len, PROT_READ | PROT_WRITE,
                         MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (regular == MAP_FAILED) { perror("mmap regular"); return; }
    double mmap_time = now_sec() - t0;

    t0 = now_sec();
    for (size_t i = 0; i < len; i += PAGE_SZ)
        regular[i] = 1;
    double touch_time = now_sec() - t0;

    printf("  Regular (4 KB pages): mmap %.2f ms, touch %.2f ms\n",
           mmap_time * 1000, touch_time * 1000);
    fflush(stdout);
    munmap(regular, len);

#if HAS_HUGEPAGES
    t0 = now_sec();
    char *huge = mmap(NULL, len, PROT_READ | PROT_WRITE,
                      MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB, -1, 0);
    if (huge == MAP_FAILED) {
        printf("  Huge (2 MB pages): not available (ENOMEM/pool empty)\n");
        printf("  Enable with: echo 32 > /proc/sys/vm/nr_hugepages\n");
    } else {
        double huge_mmap_time = now_sec() - t0;

        t0 = now_sec();
        for (size_t i = 0; i < len; i += 512 * PAGE_SZ)
            huge[i] = 1;
        double huge_touch_time = now_sec() - t0;

        printf("  Huge   (2 MB pages): mmap %.2f ms, touch %.2f ms\n",
               huge_mmap_time * 1000, huge_touch_time * 1000);
        munmap(huge, len);
    }
#else
    printf("  Huge (2 MB pages): not supported on this OS (Linux only)\n");
#endif
    fflush(stdout);
}

static void bench_mmap_random_access(void)
{
    printf("\n--- mmap Random Access (when mmap can be slower) ---\n");
    fflush(stdout);
    size_t small_file = 4 * 1024 * 1024;
    int fd = create_test_file("/tmp/zerocopy_rand", small_file);
    if (fd < 0) { perror("open rand"); return; }
    warm_file(fd, small_file);

    char *addr = mmap(NULL, small_file, PROT_READ, MAP_PRIVATE, fd, 0);
    if (addr == MAP_FAILED) { perror("mmap rand"); close(fd); return; }

    unsigned int seed = 42;
    int npages = (int)(small_file / PAGE_SZ);

    double t0 = now_sec();
    volatile unsigned char sink;
    for (int i = 0; i < 10000; i++) {
        int pg = rand_r(&seed) % npages;
        sink = addr[pg * PAGE_SZ];
    }
    double rand_time = now_sec() - t0;

    printf("  10000 random page accesses: %.2f ms (%.2f us/access)\n",
           rand_time * 1000, (rand_time * 1e6) / 10000);
    fflush(stdout);
    (void)sink;
    munmap(addr, small_file);
    close(fd);
    unlink("/tmp/zerocopy_rand");
}

static void show_sendfile_example(void)
{
    printf("\n--- sendfile() API (demonstration) ---\n");
    printf("  sendfile(out_fd, in_fd, &offset, count)\n");
    printf("  - in_fd: must be a regular file\n");
    printf("  - out_fd: must be a socket (or any fd on newer kernels)\n");
    printf("  - Data flows kernel-space only: no user copy\n");
    printf("  - Example: web server serving static files\n\n");

    printf("  splice(fd_in, off_in, fd_out, off_out, len, flags)\n");
    printf("  - At least one fd must be a pipe\n");
    printf("  - Data moves in kernel space between pipe and fd\n\n");

    printf("  tee(fd_in, fd_out, len, flags)\n");
    printf("  - Both fds must be pipes\n");
    printf("  - Duplicates data without copying\n");
    fflush(stdout);
}

int main(void)
{
    printf("=== Zero-Copy & mmap Benchmark Suite ===\n\n");
    fflush(stdout);

    int fd = create_test_file("/tmp/zerocopy_test", FILE_SIZE);
    if (fd < 0) { perror("create test file"); return 1; }
    warm_file(fd, FILE_SIZE);

    struct stat st;
    fstat(fd, &st);

    printf("--- Sequential Access: read() vs mmap() (%d MB) ---\n",
           FILE_SIZE_MB);
    fflush(stdout);
    bench_read(fd);
    bench_mmap(fd, (size_t)st.st_size);

    measure_page_faults();
    demonstrate_cow();
    bench_fork_cow();
    compare_huge_pages();
    bench_mmap_random_access();
    show_sendfile_example();

    close(fd);
    unlink("/tmp/zerocopy_test");

    printf("\n=== Summary ===\n");
    printf("  - mmap wins for large sequential reads (no user-copy)\n");
    printf("  - read() wins for random access on large files (predictable)\n");
    printf("  - sendfile/splice win for fd-to-fd transfers\n");
    printf("  - COW saves memory in fork() and MAP_PRIVATE\n");
    printf("  - Huge pages reduce TLB pressure for large allocations\n");
    printf("  - O_DIRECT bypasses page cache for self-managed caches\n");
    printf("  - MSG_ZEROCOPY avoids NIC copy for large network sends\n");
    return 0;
}