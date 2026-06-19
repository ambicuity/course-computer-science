/*
 * Reactor and Proactor Patterns — epoll, kqueue, io_uring
 * Phase 13 — Concurrent & Parallel Computing
 *
 * Two echo server implementations:
 *   1. epoll-based (reactor) — readiness notification
 *   2. io_uring-based (proactor) — completion notification
 *
 * Also includes a benchmark client for throughput comparison.
 *
 * Build:
 *   clang -std=c11 -O2 -o echo_epoll main.c -DEFOLL
 *   clang -std=c11 -O2 -luring -o echo_uring main.c -DIO_URING
 *
 * Usage:
 *   ./echo_epoll           # runs epoll server on port 8080
 *   ./echo_epoll 9090      # runs epoll server on port 9090
 *   ./echo_uring           # runs io_uring server on port 8080
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <signal.h>
#include <stdatomic.h>
#include <time.h>
#include <pthread.h>

#include <sys/socket.h>
#include <sys/epoll.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <fcntl.h>

static volatile atomic_int running = 1;

static void handle_signal(int sig) {
    (void)sig;
    atomic_store(&running, 0);
}

static int make_listen_socket(int port) {
    int fd = socket(AF_INET, SOCK_STREAM | SOCK_NONBLOCK, 0);
    if (fd < 0) { perror("socket"); return -1; }
    int opt = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    struct sockaddr_in addr = {
        .sin_family = AF_INET,
        .sin_port = htons(port),
        .sin_addr = { .s_addr = htonl(INADDR_ANY) },
    };
    if (bind(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        perror("bind"); close(fd); return -1;
    }
    if (listen(fd, 128) < 0) {
        perror("listen"); close(fd); return -1;
    }
    return fd;
}

static int set_nonblock(int fd) {
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags < 0) return -1;
    return fcntl(fd, F_SETFL, flags | O_NONBLOCK);
}

/* ── Part 1: epoll Echo Server (Reactor) ───────────────────────────────
 *
 * How epoll (reactor) works:
 *   1. epoll_create1 creates an epoll instance (red-black tree + ready list)
 *   2. epoll_ctl(ADD) registers fds with interest flags (EPOLLIN, EPOLLOUT)
 *   3. epoll_wait blocks until at least one fd is ready
 *   4. The caller then performs read/write on the ready fd
 *   5. Edge-triggered (EPOLLET) means: notify only on state transitions;
 *      you must read until EAGAIN.
 *
 * This server uses EPOLLET + EPOLLONESHOT to ensure fair dispatch:
 * after one event, the fd is removed from epoll until explicitly re-armed.
 */

/*
 * Thread-safe statistics for the epoll server.
 */
static struct {
    atomic_ulong connections;
    atomic_ulong bytes_read;
    atomic_ulong bytes_written;
    atomic_ulong errors;
} epoll_stats = { 0, 0, 0, 0 };

static void print_epoll_stats(void) {
    unsigned long c = atomic_load(&epoll_stats.connections);
    unsigned long r = atomic_load(&epoll_stats.bytes_read);
    unsigned long w = atomic_load(&epoll_stats.bytes_written);
    unsigned long e = atomic_load(&epoll_stats.errors);
    printf("[epoll] stats: conns=%lu, read=%lu, written=%lu, errors=%lu\n",
           c, r, w, e);
}

static void handle_epoll_event(int epfd, struct epoll_event *ev) {
    int fd = ev->data.fd;
    char buf[65536];

    for (;;) {
        ssize_t n = read(fd, buf, sizeof(buf));
        if (n > 0) {
            atomic_fetch_add(&epoll_stats.bytes_read, (unsigned long)n);
            ssize_t written = 0;
            while (written < n) {
                ssize_t w = write(fd, buf + written, (size_t)(n - written));
                if (w <= 0) {
                    if (errno == EAGAIN || errno == EWOULDBLOCK) {
                        continue;
                    }
                    atomic_fetch_add(&epoll_stats.errors, 1);
                    goto close_fd;
                }
                written += w;
                atomic_fetch_add(&epoll_stats.bytes_written, (unsigned long)w);
            }
        } else if (n == 0) {
            goto close_fd;
        } else {
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                struct epoll_event nev = {
                    .events = EPOLLIN | EPOLLET | EPOLLONESHOT,
                    .data.fd = fd,
                };
                epoll_ctl(epfd, EPOLL_CTL_MOD, fd, &nev);
                return;
            }
            atomic_fetch_add(&epoll_stats.errors, 1);
            goto close_fd;
        }
    }
    return;

close_fd:
    epoll_ctl(epfd, EPOLL_CTL_DEL, fd, NULL);
    close(fd);
}

/*
 * run_epoll: creates a listen socket, adds it to epoll, and runs the
 * event loop. For each incoming connection, accepts it and registers
 * the client fd for edge-triggered read events.
 */
static void run_epoll(int port) {
    int listen_fd = make_listen_socket(port);
    if (listen_fd < 0) return;

    int epfd = epoll_create1(0);
    if (epfd < 0) { perror("epoll_create1"); close(listen_fd); return; }

    struct epoll_event ev = {
        .events = EPOLLIN,
        .data.fd = listen_fd,
    };
    epoll_ctl(epfd, EPOLL_CTL_ADD, listen_fd, &ev);

    printf("[epoll] Echo server listening on port %d (pid=%d)\n", port, getpid());
    printf("[epoll] Mode: edge-triggered (EPOLLET) with EPOLLONESHOT\n");

    /* Stats printing timer thread */
    pthread_t stats_thread;
    pthread_create(&stats_thread, NULL,
        (void* (*)(void*))((void*)0), NULL);

    struct epoll_event events[128];
    while (atomic_load(&running)) {
        int n = epoll_wait(epfd, events, 128, 1000);
        if (n < 0) {
            if (errno == EINTR) continue;
            perror("epoll_wait");
            break;
        }
        for (int i = 0; i < n; i++) {
            if (events[i].data.fd == listen_fd) {
                struct sockaddr_in client_addr;
                socklen_t client_len = sizeof(client_addr);
                int client_fd = accept4(listen_fd,
                    (struct sockaddr*)&client_addr, &client_len,
                    SOCK_NONBLOCK);
                if (client_fd < 0) {
                    if (errno != EAGAIN && errno != EWOULDBLOCK)
                        perror("accept4");
                    continue;
                }
                atomic_fetch_add(&epoll_stats.connections, 1);
                struct epoll_event cev = {
                    .events = EPOLLIN | EPOLLET | EPOLLONESHOT,
                    .data.fd = client_fd,
                };
                epoll_ctl(epfd, EPOLL_CTL_ADD, client_fd, &cev);
            } else {
                handle_epoll_event(epfd, &events[i]);
            }
        }
    }

    close(epfd);
    close(listen_fd);
    print_epoll_stats();
    printf("[epoll] Shut down.\n");
}

/* ── Part 2: io_uring Echo Server (Proactor) ───────────────────────────
 *
 * How io_uring (proactor) works:
 *   1. io_uring_queue_init creates two ring buffers: SQ and CQ
 *   2. User fills SQ entries (SQEs) with operations: accept, readv, writev
 *   3. io_uring_submit notifies the kernel to process SQEs
 *   4. io_uring_wait_cqe blocks until at least one completion arrives in CQ
 *   5. Each CQE contains the result (res) and user_data to identify the op
 *   6. After processing, call io_uring_cqe_seen to advance the CQ tail
 *
 * No read()/write() syscalls. The kernel reads/writes directly into
 * the user-supplied buffers specified in the SQEs.
 */

#ifdef IO_URING
#include <liburing.h>
#include <sys/uio.h>

#define MAX_CONNS 4096
#define BUF_SIZE  4096

typedef enum { OP_NONE, OP_ACCEPT, OP_READ, OP_WRITE } io_op;

typedef struct {
    int fd;
    io_op op;
    struct iovec iov;
    char buf[BUF_SIZE];
} conn_state;

static conn_state conns[MAX_CONNS];
static int nconns = 0;
static atomic_ulong uring_bytes_read = 0;
static atomic_ulong uring_ops = 0;

static int conn_alloc(void) {
    if (nconns >= MAX_CONNS) return -1;
    int id = __sync_fetch_and_add(&nconns, 1);
    if (id >= MAX_CONNS) return -1;
    conns[id].fd = -1;
    conns[id].op = OP_NONE;
    return id;
}

static void conn_free(int id) {
    if (id >= 0 && id < MAX_CONNS) {
        if (conns[id].fd >= 0) close(conns[id].fd);
        conns[id].fd = -1;
        conns[id].op = OP_NONE;
    }
}

/*
 * Submit an accept SQE. When completed, the CQE's res field will
 * contain the new client file descriptor (or a negative error).
 */
static void submit_accept(struct io_uring *ring, int listen_fd) {
    struct io_uring_sqe *sqe = io_uring_get_sqe(ring);
    if (!sqe) {
        fprintf(stderr, "[io_uring] get_sqe failed (accept)\n");
        return;
    }
    io_uring_prep_accept(sqe, listen_fd, NULL, NULL, 0);
    sqe->user_data = (uint64_t)(intptr_t)-1;
}

/*
 * Submit a readv SQE. On completion, the CQE's res is the number of
 * bytes read (0 = EOF, negative = error). The data lands in conns[id].buf.
 */
static void submit_read(struct io_uring *ring, int conn_id) {
    conn_state *c = &conns[conn_id];
    struct io_uring_sqe *sqe = io_uring_get_sqe(ring);
    if (!sqe) {
        fprintf(stderr, "[io_uring] get_sqe failed (read)\n");
        return;
    }
    c->iov.iov_base = c->buf;
    c->iov.iov_len = BUF_SIZE;
    c->op = OP_READ;
    io_uring_prep_readv(sqe, c->fd, &c->iov, 1, 0);
    sqe->user_data = (uint64_t)(intptr_t)conn_id;
}

/*
 * Submit a writev SQE. On completion, the CQE's res is the number of
 * bytes written (or negative error). The data comes from conns[id].buf.
 */
static void submit_write(struct io_uring *ring, int conn_id, size_t len) {
    conn_state *c = &conns[conn_id];
    struct io_uring_sqe *sqe = io_uring_get_sqe(ring);
    if (!sqe) {
        fprintf(stderr, "[io_uring] get_sqe failed (write)\n");
        return;
    }
    c->iov.iov_base = c->buf;
    c->iov.iov_len = len;
    c->op = OP_WRITE;
    io_uring_prep_writev(sqe, c->fd, &c->iov, 1, 0);
    sqe->user_data = (uint64_t)(intptr_t)conn_id;
}

/*
 * Submit a batch of up to 8 available read operations to fill the
 * submission queue before calling io_uring_submit. This amortizes
 * the cost of the syscall across multiple operations.
 */
static void submit_batch_reads(struct io_uring *ring, int *ids, int count) {
    for (int i = 0; i < count; i++) {
        submit_read(ring, ids[i]);
    }
}

/*
 * run_io_uring: the io_uring echo server event loop.
 * 1. Submit an accept SQE
 * 2. Loop: wait for CQE completions
 * 3. On accept completion: allocate a conn_state, submit a read for it
 * 4. On read completion: submit a write (echo) or close on EOF
 * 5. On write completion: submit the next read
 * 6. Submit a new accept SQE after each accept
 */
static void run_io_uring(int port) {
    struct io_uring ring;
    int ret = io_uring_queue_init(512, &ring, 0);
    if (ret) {
        fprintf(stderr, "[io_uring] queue_init failed: %s\n", strerror(-ret));
        return;
    }

    int listen_fd = make_listen_socket(port);
    if (listen_fd < 0) { io_uring_queue_exit(&ring); return; }

    printf("[io_uring] Echo server listening on port %d (pid=%d)\n", port, getpid());
    printf("[io_uring] SQ entries: 512, kernel %s\n",
           "5.1+ (SQPOLL available in 5.11+)");

    submit_accept(&ring, listen_fd);
    io_uring_submit(&ring);

    struct io_uring_cqe *cqe;
    unsigned long total_read = 0;
    unsigned long total_ops = 0;

    while (atomic_load(&running)) {
        ret = io_uring_wait_cqe(&ring, &cqe);
        if (ret < 0) {
            if (errno == EINTR) continue;
            fprintf(stderr, "[io_uring] wait_cqe failed: %s\n", strerror(-ret));
            break;
        }

        intptr_t user_data = (intptr_t)cqe->user_data;
        int res = cqe->res;
        io_uring_cqe_seen(&ring, cqe);
        total_ops++;

        if (user_data == -1) {
            /* --- Accept completed --- */
            if (res < 0) {
                if (res == -EAGAIN) {
                    submit_accept(&ring, listen_fd);
                    io_uring_submit(&ring);
                } else {
                    fprintf(stderr, "[io_uring] accept error: %s\n",
                            strerror(-res));
                }
                continue;
            }
            int client_fd = res;
            set_nonblock(client_fd);

            int conn_id = conn_alloc();
            if (conn_id < 0) {
                fprintf(stderr, "[io_uring] max conns (%d) reached\n", MAX_CONNS);
                close(client_fd);
            } else {
                conns[conn_id].fd = client_fd;
                submit_read(&ring, conn_id);
            }

            submit_accept(&ring, listen_fd);
            io_uring_submit(&ring);

        } else {
            /* --- Read or Write completed --- */
            int conn_id = (int)user_data;
            if (conn_id < 0 || conn_id >= MAX_CONNS) continue;
            conn_state *c = &conns[conn_id];

            if (c->op == OP_READ) {
                if (res <= 0) {
                    conn_free(conn_id);
                } else {
                    total_read += (unsigned long)res;
                    submit_write(&ring, conn_id, (size_t)res);
                    io_uring_submit(&ring);
                }
            } else if (c->op == OP_WRITE) {
                if (res < 0) {
                    conn_free(conn_id);
                } else {
                    submit_read(&ring, conn_id);
                    io_uring_submit(&ring);
                }
            }
        }
    }

    for (int i = 0; i < nconns; i++) conn_free(i);
    close(listen_fd);
    io_uring_queue_exit(&ring);
    printf("[io_uring] total_read=%lu, total_ops=%lu\n",
           total_read, total_ops);
    printf("[io_uring] Shut down.\n");
}
#endif /* IO_URING */

/* ── Benchmark Client (for C servers) ──────────────────────────────────
 *
 * Opens nconns connections, sends nrequests messages on each, and
 * reports aggregate throughput.
 */

static void bench_client_worker(
    const char *host, int port, int nrequests,
    double *out_elapsed, long *out_bytes)
{
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) { perror("socket"); *out_elapsed = -1; return; }

    struct sockaddr_in addr = {
        .sin_family = AF_INET,
        .sin_port = htons(port),
    };
    inet_pton(AF_INET, host, &addr.sin_addr);

    if (connect(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        perror("connect"); close(fd); *out_elapsed = -1; return;
    }

    const char *msg = "hello";
    size_t msglen = strlen(msg);
    char buf[4096];
    long total = 0;

    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);

    for (int i = 0; i < nrequests; i++) {
        ssize_t sent = write(fd, msg, msglen);
        if (sent <= 0) { perror("write"); break; }
        ssize_t n = read(fd, buf, msglen);
        if (n <= 0) { perror("read"); break; }
        total += n;
    }

    clock_gettime(CLOCK_MONOTONIC, &t1);
    close(fd);

    double secs = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) / 1e9;
    *out_elapsed = secs;
    *out_bytes = total;
}

static void benchmark_client(const char *host, int port,
                             int nconns, int nrequests)
{
    printf("=== Benchmark ===\n");
    printf("Server: %s:%d\n", host, port);
    printf("Connections: %d\n", nconns);
    printf("Requests per connection: %d\n", nrequests);

    double total_time = 0;
    long total_bytes = 0;

    for (int i = 0; i < nconns; i++) {
        double elapsed;
        long bytes;
        bench_client_worker(host, port, nrequests, &elapsed, &bytes);
        if (elapsed < 0) {
            fprintf(stderr, "Worker %d failed\n", i);
            continue;
        }
        total_time += elapsed;
        total_bytes += bytes;
    }

    long total_ops = (long)nconns * nrequests;
    double avg_time = total_time / nconns;
    double throughput = total_ops / avg_time;

    printf("Total requests: %ld\n", total_ops);
    printf("Total bytes:    %ld\n", total_bytes);
    printf("Avg time/conn:  %.3f ms\n", avg_time * 1000);
    printf("Throughput:     %.0f req/s\n", throughput);
    printf("Bandwidth:      %.1f MB/s\n",
           total_bytes / (1024.0 * 1024.0) / (total_time / nconns));
}

/* ── kqueue notes (macOS/BSD) ──────────────────────────────────────────
 *
 * On macOS/BSD, epoll is not available. Use kqueue instead:
 *
 *   int kq = kqueue();
 *   struct kevent change;
 *   EV_SET(&change, fd, EVFILT_READ, EV_ADD | EV_ENABLE, 0, 0, 0);
 *   kevent(kq, &change, 1, NULL, 0, NULL);
 *
 *   struct kevent events[128];
 *   int n = kevent(kq, NULL, 0, events, 128, NULL);
 *   // events[i].filter == EVFILT_READ  → socket is readable
 *   // events[i].data holds the number of bytes available (no ioctl FIONREAD needed)
 *
 * kqueue is more capable than epoll — it can monitor timers, signals,
 * process events, and file modifications — but the reactor pattern is
 * identical: readiness notification, then you read/write.
 *
 * This lesson focuses on epoll and io_uring (Linux). For macOS, see
 * kqueue docs: man 2 kqueue, man 2 kevent.
 */

/* ── Main ────────────────────────────────────────────────────────────── */

int main(int argc, char **argv) {
    signal(SIGINT, handle_signal);
    signal(SIGTERM, handle_signal);

    int port = 8080;
    const char *mode = "server";

    if (argc > 1) port = atoi(argv[1]);
    if (argc > 2) mode = argv[2];
    if (port <= 0 || port > 65535) {
        fprintf(stderr, "Usage: %s [port] [server|bench [nconns [nreqs]]]\n", argv[0]);
        return 1;
    }

    if (strcmp(mode, "bench") == 0 || strcmp(mode, "client") == 0) {
        int nconns = (argc > 3) ? atoi(argv[3]) : 10;
        int nreqs  = (argc > 4) ? atoi(argv[4]) : 100;
        benchmark_client("127.0.0.1", port, nconns, nreqs);
        return 0;
    }

#if defined(IO_URING)
    printf("=== io_uring Echo Server (Proactor) ===\n");
    run_io_uring(port);
#elif defined(EPOLL)
    printf("=== epoll Echo Server (Reactor) ===\n");
    run_epoll(port);
#else
    printf("=== Running both servers sequentially ===\n");
    printf("Define -DEFOLL or -DIO_URING to select one.\n\n");

    printf("--- epoll (Reactor) ---\n");
    run_epoll(port);

    printf("\n--- Restarting for io_uring ---\n");
    printf("(Not compiled with -DIO_URING, skipped)\n");
#endif

    return 0;
}
