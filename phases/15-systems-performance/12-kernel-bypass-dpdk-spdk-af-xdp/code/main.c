/*
 * Kernel Bypass — DPDK, SPDK, AF_XDP
 * Phase 15 — Systems Performance
 *
 * Conceptual benchmark: kernel syscall overhead vs userspace polling.
 * Measures throughput and context-switch cost for:
 *   1. read() from /dev/urandom (kernel syscall path)
 *   2. mmap() + polling of a buffer (userspace zero-copy analog)
 *
 * This is NOT full DPDK — it demonstrates WHY kernel bypass matters
 * by quantifying the overhead the kernel adds.
 *
 * No external libraries beyond POSIX.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <time.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sched.h>
#include <pthread.h>

#define BUF_SIZE        (1 << 20)
#define ITERATIONS_UART 1000
#define ITERATIONS_POLL 1000
#define POLL_BATCH      256

static double timespec_diff_sec(struct timespec *start, struct timespec *end)
{
    return (end->tv_sec - start->tv_sec) +
           (end->tv_nsec - start->tv_nsec) / 1e9;
}

static long read_context_switches(void)
{
    FILE *f = fopen("/proc/self/status", "r");
    if (!f)
        return -1;

    char line[256];
    long voluntary = -1, nonvoluntary = -1;

    while (fgets(line, sizeof(line), f)) {
        if (strncmp(line, "voluntary_ctxt_switches:", 23) == 0)
            voluntary = atol(line + 23);
        if (strncmp(line, "nonvoluntary_ctxt_switches:", 28) == 0)
            nonvoluntary = atol(line + 28);
    }

    fclose(f);
    if (voluntary < 0 || nonvoluntary < 0)
        return -1;
    return voluntary + nonvoluntary;
}

static unsigned long xorshift64(unsigned long *state)
{
    unsigned long x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    return x;
}

static int benchmark_syscall_read(void)
{
    int fd = open("/dev/urandom", O_RDONLY);
    if (fd < 0) {
        perror("open /dev/urandom");
        return -1;
    }

    unsigned char *buf = malloc(BUF_SIZE);
    if (!buf) {
        close(fd);
        return -1;
    }

    long cs_before = read_context_switches();
    struct timespec t_start, t_end;
    clock_gettime(CLOCK_MONOTONIC, &t_start);

    size_t total_bytes = 0;
    for (int i = 0; i < ITERATIONS_UART; i++) {
        ssize_t n = read(fd, buf, BUF_SIZE);
        if (n < 0) {
            perror("read");
            free(buf);
            close(fd);
            return -1;
        }
        total_bytes += (size_t)n;
        volatile unsigned char sink = buf[0];
        (void)sink;
    }

    clock_gettime(CLOCK_MONOTONIC, &t_end);
    long cs_after = read_context_switches();

    double elapsed = timespec_diff_sec(&t_start, &t_end);
    double throughput_gbps = (total_bytes * 8.0) / (elapsed * 1e9);
    double throughput_mbps = (total_bytes / (1024.0 * 1024.0)) / elapsed;

    printf("=== Kernel Syscall Path: read(/dev/urandom) ===\n");
    printf("  Iterations:       %d\n", ITERATIONS_UART);
    printf("  Buffer size:      %d bytes\n", BUF_SIZE);
    printf("  Total bytes:      %zu\n", total_bytes);
    printf("  Elapsed:          %.4f s\n", elapsed);
    printf("  Throughput:       %.2f MB/s (%.2f Gbps)\n", throughput_mbps, throughput_gbps);
    printf("  Syscalls:         %d (one per read)\n", ITERATIONS_UART);
    printf("  Context switches:  %ld\n", cs_after - cs_before);
    printf("\n");

    free(buf);
    close(fd);
    return 0;
}

static int benchmark_mmap_poll(void)
{
    unsigned char *buf = mmap(NULL, BUF_SIZE, PROT_READ | PROT_WRITE,
                              MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (buf == MAP_FAILED) {
        perror("mmap");
        return -1;
    }

    unsigned long rng_state = (unsigned long)(uintptr_t)buf | 0xDEADBEEF;
    for (size_t i = 0; i < BUF_SIZE; i++)
        buf[i] = (unsigned char)(xorshift64(&rng_state) & 0xFF);

    long cs_before = read_context_switches();
    struct timespec t_start, t_end;
    clock_gettime(CLOCK_MONOTONIC, &t_start);

    size_t total_bytes = 0;
    volatile unsigned char sink = 0;

    for (int i = 0; i < ITERATIONS_POLL; i++) {
        for (int j = 0; j < POLL_BATCH; j++) {
            size_t idx = xorshift64(&rng_state) % BUF_SIZE;
            sink = buf[idx];
        }
        total_bytes += POLL_BATCH;
    }

    clock_gettime(CLOCK_MONOTONIC, &t_end);
    long cs_after = read_context_switches();

    double elapsed = timespec_diff_sec(&t_start, &t_end);
    double operations_per_sec = total_bytes / elapsed;

    printf("=== Userspace Poll Path: mmap + polling ===\n");
    printf("  Iterations:        %d x %d = %d reads\n",
           ITERATIONS_POLL, POLL_BATCH, ITERATIONS_POLL * POLL_BATCH);
    printf("  Buffer size:       %d bytes (mmap'd)\n", BUF_SIZE);
    printf("  Total reads:       %zu\n", total_bytes);
    printf("  Elapsed:           %.6f s\n", elapsed);
    printf("  Throughput:        %.0f reads/sec\n", operations_per_sec);
    printf("  Syscalls:          0 (pure userspace memory access)\n");
    printf("  Context switches:  %ld\n", cs_after - cs_before);
    printf("\n");

    (void)sink;
    munmap(buf, BUF_SIZE);
    return 0;
}

static int benchmark_syscall_overhead_per_op(void)
{
    int fd = open("/dev/urandom", O_RDONLY);
    if (fd < 0) {
        perror("open /dev/urandom");
        return -1;
    }

    unsigned char small_buf[1];

    struct timespec t_start, t_end;
    clock_gettime(CLOCK_MONOTONIC, &t_start);

    for (int i = 0; i < 10000; i++) {
        read(fd, small_buf, 1);
    }

    clock_gettime(CLOCK_MONOTONIC, &t_end);
    close(fd);

    double elapsed = timespec_diff_sec(&t_start, &t_end);
    double ns_per_call = (elapsed / 10000.0) * 1e9;

    printf("=== Syscall Overhead: 1-byte read() x 10000 ===\n");
    printf("  Total elapsed:     %.4f s\n", elapsed);
    printf("  Time per syscall:  %.0f ns\n", ns_per_call);
    printf("  (This is the MINIMUM cost of going through the kernel\n");
    printf("   for each I/O operation — the overhead kernel bypass eliminates)\n");
    printf("\n");

    return 0;
}

static void print_architecture_comparison(void)
{
    printf("=== Kernel Bypass Architecture Comparison ===\n\n");
    printf("  Traditional Kernel Path:\n");
    printf("    App -> syscall -> kernel -> driver -> hardware\n");
    printf("    App <- syscall <- kernel <- driver <- hardware\n");
    printf("    Cost: context switch + memcpy + interrupt per packet\n\n");
    printf("  DPDK Path:\n");
    printf("    App -> PMD (poll) -> NIC (via huge pages + VFIO)\n");
    printf("    App <- PMD (poll) <- NIC (zero-copy from DMA)\n");
    printf("    Cost: poll loop (no syscall, no interrupt, no memcpy)\n\n");
    printf("  AF_XDP Path:\n");
    printf("    App -> BPF program -> XDP_REDIRECT -> UMEM (zero-copy)\n");
    printf("    App <- UMEM fill ring <- NIC DMA\n");
    printf("    Cost: BPF execution (~50-100ns) + ring poll (no syscall per pkt)\n\n");
    printf("  io_uring Path (middle ground):\n");
    printf("    App -> ring buffer -> kernel (batched) -> driver\n");
    printf("    App <- ring buffer <- kernel (batched) <- driver\n");
    printf("    Cost: amortized syscall (1 per batch, not per op)\n\n");
}

int main(void)
{
    printf("========================================\n");
    printf(" Kernel Bypass Conceptual Benchmark\n");
    printf(" Phase 15 — Systems Performance\n");
    printf("========================================\n\n");

    print_architecture_comparison();

    printf("----------------------------------------\n");
    printf(" Running benchmarks...\n");
    printf("----------------------------------------\n\n");

    if (benchmark_syscall_read() < 0)
        fprintf(stderr, "Syscall read benchmark failed\n");

    if (benchmark_mmap_poll() < 0)
        fprintf(stderr, "Mmap poll benchmark failed\n");

    if (benchmark_syscall_overhead_per_op() < 0)
        fprintf(stderr, "Syscall overhead benchmark failed\n");

    printf("========================================\n");
    printf(" Key Takeaway:\n");
    printf("   Every syscall costs ~100-200ns minimum.\n");
    printf("   Every context switch costs ~1-5us.\n");
    printf("   Every memcpy from kernel to userspace\n");
    printf("   costs bandwidth proportional to packet size.\n");
    printf("   Kernel bypass eliminates ALL of these\n");
    printf("   for data-plane operations.\n");
    printf("========================================\n");

    return 0;
}