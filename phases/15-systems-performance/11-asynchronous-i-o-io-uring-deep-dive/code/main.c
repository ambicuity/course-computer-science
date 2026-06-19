/*
 * Asynchronous I/O — io_uring Deep Dive
 * Phase 15 — Systems Programming & Performance
 *
 * Benchmark: sequential read() vs io_uring batched read on a file.
 * Also demonstrates fixed files and buffer registration.
 *
 * Build (Linux with liburing):
 *   gcc -O2 -o iouring_bench main.c -luring
 *
 * Build (other platforms — falls back to sequential-only):
 *   gcc -O2 -o iouring_bench main.c
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <time.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>

#ifdef __linux__
#include <liburing.h>
#define HAS_IOURING 1
#else
#define HAS_IOURING 0
#endif

#define NUM_BLOCKS   128
#define BLOCK_SIZE   4096
#define QUEUE_DEPTH  128

static double now_sec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

static int sequential_read(int fd, size_t file_size) {
    char buf[BLOCK_SIZE];
    size_t blocks = file_size / BLOCK_SIZE;
    if (blocks == 0) blocks = 1;

    double t0 = now_sec();
    for (size_t i = 0; i < blocks; i++) {
        ssize_t n = pread(fd, buf, BLOCK_SIZE, (off_t)(i * BLOCK_SIZE));
        if (n < 0) {
            perror("pread");
            return -1;
        }
    }
    double t1 = now_sec();
    printf("  sequential: %.3f ms for %zu blocks\n",
           (t1 - t0) * 1000.0, blocks);
    return 0;
}

#if HAS_IOURING
static int iouring_read(int fd, size_t file_size) {
    struct io_uring ring;
    int ret;
    char *bufs[QUEUE_DEPTH];
    size_t blocks = file_size / BLOCK_SIZE;
    if (blocks == 0) blocks = 1;

    ret = io_uring_queue_init(QUEUE_DEPTH, &ring, 0);
    if (ret < 0) {
        fprintf(stderr, "io_uring_queue_init: %s\n", strerror(-ret));
        return -1;
    }

    for (int i = 0; i < QUEUE_DEPTH; i++) {
        bufs[i] = aligned_alloc(4096, BLOCK_SIZE);
        if (!bufs[i]) {
            perror("aligned_alloc");
            io_uring_queue_exit(&ring);
            return -1;
        }
    }

    double t0 = now_sec();

    size_t submitted = 0;
    size_t completed = 0;

    while (completed < blocks) {
        unsigned int batch = 0;
        while (submitted < blocks && batch < QUEUE_DEPTH) {
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            if (!sqe) break;
            size_t idx = submitted % QUEUE_DEPTH;
            io_uring_prep_read(sqe, fd, bufs[idx], BLOCK_SIZE,
                                (off_t)(submitted * BLOCK_SIZE));
            io_uring_sqe_set_data64(sqe, submitted);
            submitted++;
            batch++;
        }

        ret = io_uring_submit(&ring);
        if (ret < 0) {
            fprintf(stderr, "io_uring_submit: %s\n", strerror(-ret));
            goto cleanup;
        }

        for (unsigned int i = 0; i < batch; i++) {
            struct io_uring_cqe *cqe;
            ret = io_uring_wait_cqe(&ring, &cqe);
            if (ret < 0) {
                fprintf(stderr, "io_uring_wait_cqe: %s\n", strerror(-ret));
                goto cleanup;
            }
            if (cqe->res < 0) {
                fprintf(stderr, "io_uring read error: %s\n",
                        strerror(-cqe->res));
                io_uring_cqe_seen(&ring, cqe);
                goto cleanup;
            }
            io_uring_cqe_seen(&ring, cqe);
            completed++;
        }
    }

    double t1 = now_sec();
    printf("  io_uring:   %.3f ms for %zu blocks\n",
           (t1 - t0) * 1000.0, blocks);

cleanup:
    for (int i = 0; i < QUEUE_DEPTH; i++) free(bufs[i]);
    io_uring_queue_exit(&ring);
    return (completed == blocks) ? 0 : -1;
}

static int iouring_fixed_read(int fd, size_t file_size) {
    struct io_uring ring;
    int ret;
    size_t blocks = file_size / BLOCK_SIZE;
    if (blocks == 0) blocks = 1;

    ret = io_uring_queue_init(QUEUE_DEPTH, &ring, 0);
    if (ret < 0) {
        fprintf(stderr, "io_uring_queue_init: %s\n", strerror(-ret));
        return -1;
    }

    char *bigbuf = aligned_alloc(4096, BLOCK_SIZE);
    if (!bigbuf) { perror("aligned_alloc"); io_uring_queue_exit(&ring); return -1; }

    struct iovec iov = { .iov_base = bigbuf, .iov_len = BLOCK_SIZE };
    ret = io_uring_register_buffers(&ring, &iov, 1);
    if (ret < 0) {
        fprintf(stderr, "register_buffers: %s (skipping fixed test)\n",
                strerror(-ret));
        free(bigbuf);
        io_uring_queue_exit(&ring);
        return 0;
    }

    ret = io_uring_register_files(&ring, &fd, 1);
    if (ret < 0) {
        fprintf(stderr, "register_files: %s (skipping fixed test)\n",
                strerror(-ret));
        io_uring_unregister_buffers(&ring);
        free(bigbuf);
        io_uring_queue_exit(&ring);
        return 0;
    }

    double t0 = now_sec();
    size_t submitted = 0;
    size_t completed = 0;

    while (completed < blocks) {
        unsigned int batch = 0;
        while (submitted < blocks && batch < QUEUE_DEPTH) {
            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            if (!sqe) break;
            io_uring_prep_read_fixed(sqe, 0, bigbuf, BLOCK_SIZE,
                                      (off_t)(submitted * BLOCK_SIZE), 0);
            sqe->flags |= IOSQE_FIXED_FILE;
            io_uring_sqe_set_data64(sqe, submitted);
            submitted++;
            batch++;
        }

        ret = io_uring_submit(&ring);
        if (ret < 0) {
            fprintf(stderr, "submit: %s\n", strerror(-ret));
            goto fixed_cleanup;
        }

        for (unsigned int i = 0; i < batch; i++) {
            struct io_uring_cqe *cqe;
            ret = io_uring_wait_cqe(&ring, &cqe);
            if (ret < 0) {
                fprintf(stderr, "wait_cqe: %s\n", strerror(-ret));
                goto fixed_cleanup;
            }
            if (cqe->res < 0) {
                fprintf(stderr, "fixed read error: %s\n", strerror(-cqe->res));
                io_uring_cqe_seen(&ring, cqe);
                goto fixed_cleanup;
            }
            io_uring_cqe_seen(&ring, cqe);
            completed++;
        }
    }

    double t1 = now_sec();
    printf("  io_uring (fixed): %.3f ms for %zu blocks\n",
           (t1 - t0) * 1000.0, blocks);

fixed_cleanup:
    io_uring_unregister_files(&ring);
    io_uring_unregister_buffers(&ring);
    free(bigbuf);
    io_uring_queue_exit(&ring);
    return (completed == blocks) ? 0 : -1;
}
#endif

int main(int argc, char *argv[]) {
    const char *path = argc > 1 ? argv[1] : "/etc/hostname";
    int fd = open(path, O_RDONLY);
    if (fd < 0) {
        perror("open");
        return 1;
    }

    struct stat st;
    if (fstat(fd, &st) < 0) {
        perror("fstat");
        close(fd);
        return 1;
    }
    size_t file_size = (size_t)st.st_size;
    printf("File: %s (%zu bytes, %zu blocks)\n\n",
           path, file_size, file_size / BLOCK_SIZE ?: 1);

    printf("Benchmark 1: sequential pread()\n");
    sequential_read(fd, file_size);

#if HAS_IOURING
    printf("\nBenchmark 2: io_uring batched read\n");
    iouring_read(fd, file_size);

    printf("\nBenchmark 3: io_uring fixed buffers + fixed files\n");
    iouring_fixed_read(fd, file_size);
#else
    printf("\n(io_uring not available on this platform — skipping benchmarks 2 and 3)\n");
#endif

    close(fd);
    return 0;
}