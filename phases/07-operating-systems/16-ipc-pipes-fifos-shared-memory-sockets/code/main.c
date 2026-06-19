#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <sys/ipc.h>
#include <sys/shm.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/time.h>
#include <sched.h>
#include <signal.h>
#include <errno.h>
#include <time.h>

/*
 * IPC demos: pipe, FIFO, shared memory, Unix domain socket.
 * Compile: gcc -o ipc_demo main.c
 * Run:     ./ipc_demo
 *
 * Note: Some IPC mechanisms (shmget, etc.) may require specific
 * system headers. Shared memory demo uses POSIX semaphores.
 */

/* ---------- pipe_demo ---------- */
static void pipe_demo(void) {
    printf("=== Pipe Demo ===\n\n");

    int fd[2];
    if (pipe(fd) < 0) {
        perror("pipe");
        return;
    }

    pid_t pid = fork();
    if (pid < 0) {
        perror("fork");
        close(fd[0]);
        close(fd[1]);
        return;
    }

    if (pid == 0) {
        /* Child: read from pipe */
        close(fd[1]);
        char buf[256];
        ssize_t n = read(fd[0], buf, sizeof(buf) - 1);
        if (n > 0) {
            buf[n] = '\0';
            printf("CHILD:  received '%s' via pipe\n", buf);
        }
        close(fd[0]);
        exit(0);
    } else {
        /* Parent: write to pipe */
        close(fd[0]);
        const char *msg = "Hello from parent via pipe!";
        printf("PARENT: sending '%s'\n", msg);
        write(fd[1], msg, strlen(msg));
        close(fd[1]);

        int status;
        waitpid(pid, &status, 0);
        printf("PARENT: child done\n\n");
    }
}

/* ---------- fifo_demo ---------- */
static void fifo_demo(void) {
    printf("=== FIFO (Named Pipe) Demo ===\n\n");

    const char *fifo_path = "/tmp/ipc_demo_fifo";
    unlink(fifo_path);  /* remove stale FIFO if any */

    if (mkfifo(fifo_path, 0666) < 0) {
        perror("mkfifo");
        return;
    }

    pid_t pid = fork();
    if (pid < 0) {
        perror("fork");
        unlink(fifo_path);
        return;
    }

    if (pid == 0) {
        /* Child: read from FIFO */
        int fd = open(fifo_path, O_RDONLY);
        if (fd < 0) {
            perror("child open");
            exit(1);
        }
        char buf[256];
        ssize_t n = read(fd, buf, sizeof(buf) - 1);
        if (n > 0) {
            buf[n] = '\0';
            printf("CHILD:  received '%s' via FIFO\n", buf);
        }
        close(fd);
        exit(0);
    } else {
        /* Parent: write to FIFO */
        int fd = open(fifo_path, O_WRONLY);
        if (fd < 0) {
            perror("parent open");
            unlink(fifo_path);
            return;
        }
        const char *msg = "Hello from parent via FIFO!";
        printf("PARENT: sending '%s'\n", msg);
        write(fd, msg, strlen(msg));
        close(fd);

        int status;
        waitpid(pid, &status, 0);

        unlink(fifo_path);
        printf("PARENT: child done, FIFO removed\n\n");
    }
}

/* ---------- shm_demo ---------- */
#define SHM_SIZE 256

static void shm_demo(void) {
    printf("=== Shared Memory Demo ===\n\n");

    /* Use POSIX shared memory for better cross-platform support */
    int shm_fd = shm_open("/ipc_demo_shm", O_CREAT | O_RDWR, 0666);
    if (shm_fd < 0) {
        perror("shm_open");
        return;
    }
    if (ftruncate(shm_fd, SHM_SIZE) < 0) {
        perror("ftruncate");
        close(shm_fd);
        shm_unlink("/ipc_demo_shm");
        return;
    }
    char *shm = mmap(NULL, SHM_SIZE, PROT_READ | PROT_WRITE,
                     MAP_SHARED, shm_fd, 0);
    if (shm == MAP_FAILED) {
        perror("mmap");
        close(shm_fd);
        shm_unlink("/ipc_demo_shm");
        return;
    }
    close(shm_fd);

    /* Use a pipe to synchronize: child waits for parent to write */
    int sync[2];
    pipe(sync);

    pid_t pid = fork();
    if (pid < 0) {
        perror("fork");
        munmap(shm, SHM_SIZE);
        shm_unlink("/ipc_demo_shm");
        return;
    }

    if (pid == 0) {
        /* Child: wait for parent to signal, then read shared memory */
        close(sync[1]);
        char c;
        read(sync[0], &c, 1);
        close(sync[0]);

        printf("CHILD:  read from shared memory: '%s'\n", shm);
        munmap(shm, SHM_SIZE);
        _exit(0);
    } else {
        /* Parent: write to shared memory, then signal child */
        close(sync[0]);
        const char *msg = "Hello from parent via shared memory!";
        printf("PARENT: writing '%s' to shared memory\n", msg);
        memcpy(shm, msg, strlen(msg) + 1);

        /* Signal child that data is ready */
        write(sync[1], "x", 1);
        close(sync[1]);

        int status;
        waitpid(pid, &status, 0);

        /* Cleanup */
        munmap(shm, SHM_SIZE);
        shm_unlink("/ipc_demo_shm");
        printf("PARENT: child done, shared memory removed\n\n");
    }
}

/* ---------- socket_demo ---------- */
#define SOCK_PATH "/tmp/ipc_demo_socket"

static void socket_demo(void) {
    printf("=== Unix Domain Socket Demo ===\n\n");

    unlink(SOCK_PATH);

    int sv[2];
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, sv) < 0) {
        perror("socketpair");
        return;
    }

    pid_t pid = fork();
    if (pid < 0) {
        perror("fork");
        close(sv[0]);
        close(sv[1]);
        return;
    }

    if (pid == 0) {
        /* Child: write to sv[1], read reply from sv[1] */
        close(sv[0]);
        const char *msg = "Hello from child via socket!";
        printf("CHILD:  sending '%s'\n", msg);
        write(sv[1], msg, strlen(msg));

        char buf[256];
        ssize_t n = read(sv[1], buf, sizeof(buf) - 1);
        if (n > 0) {
            buf[n] = '\0';
            printf("CHILD:  received '%s'\n", buf);
        }
        close(sv[1]);
        exit(0);
    } else {
        /* Parent: read from sv[0], write reply to sv[0] */
        close(sv[1]);
        char buf[256];
        ssize_t n = read(sv[0], buf, sizeof(buf) - 1);
        if (n > 0) {
            buf[n] = '\0';
            printf("PARENT: received '%s'\n", buf);
        }

        const char *reply = "Hello from parent via socket!";
        printf("PARENT: sending '%s'\n", reply);
        write(sv[0], reply, strlen(reply));
        close(sv[0]);

        int status;
        waitpid(pid, &status, 0);
        unlink(SOCK_PATH);
        printf("PARENT: child done\n\n");
    }
}

/* ---------- perf_compare ---------- */

/* Helper: get time in microseconds */
static long long time_us(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (long long)tv.tv_sec * 1000000 + tv.tv_usec;
}

#define PERF_ITERATIONS 1000
#define PERF_MSG "x"

static void perf_pipe(void) {
    int fd[2];
    pipe(fd);
    pid_t pid = fork();

    if (pid == 0) {
        close(fd[1]);
        char buf[8];
        for (int i = 0; i < PERF_ITERATIONS; i++) {
            read(fd[0], buf, 1);
        }
        close(fd[0]);
        exit(0);
    } else {
        close(fd[0]);
        long long start = time_us();
        for (int i = 0; i < PERF_ITERATIONS; i++) {
            write(fd[1], PERF_MSG, 1);
        }
        close(fd[1]);
        int status;
        waitpid(pid, &status, 0);
        long long elapsed = time_us() - start;
        printf("  Pipe:          %lld us total, %lld us/iteration\n",
               elapsed, elapsed / PERF_ITERATIONS);
    }
}

static void perf_fifo(void) {
    const char *path = "/tmp/perf_fifo";
    unlink(path);
    mkfifo(path, 0666);

    pid_t pid = fork();
    if (pid == 0) {
        int fd = open(path, O_RDONLY);
        char buf[8];
        for (int i = 0; i < PERF_ITERATIONS; i++) {
            read(fd, buf, 1);
        }
        close(fd);
        unlink(path);
        exit(0);
    } else {
        int fd = open(path, O_WRONLY);
        long long start = time_us();
        for (int i = 0; i < PERF_ITERATIONS; i++) {
            write(fd, PERF_MSG, 1);
        }
        close(fd);
        int status;
        waitpid(pid, &status, 0);
        long long elapsed = time_us() - start;
        printf("  FIFO:          %lld us total, %lld us/iteration\n",
               elapsed, elapsed / PERF_ITERATIONS);
    }
}

static void perf_shm(void) {
    int shm_fd = shm_open("/perf_shm", O_CREAT | O_RDWR, 0666);
    ftruncate(shm_fd, 8);
    char *shm = mmap(NULL, 8, PROT_READ | PROT_WRITE, MAP_SHARED, shm_fd, 0);
    close(shm_fd);

    int iterations = PERF_ITERATIONS;
    shm[0] = 0;
    pid_t pid = fork();
    if (pid == 0) {
        for (int i = 0; i < iterations; i++) {
            while (__atomic_load_n(&shm[0], __ATOMIC_ACQUIRE) != 1) { sched_yield(); }
            __atomic_store_n(&shm[0], 0, __ATOMIC_RELEASE);
        }
        munmap(shm, 8);
        _exit(0);
    } else {
        long long start = time_us();
        for (int i = 0; i < iterations; i++) {
            __atomic_store_n(&shm[0], 1, __ATOMIC_RELEASE);
            while (__atomic_load_n(&shm[0], __ATOMIC_ACQUIRE) != 0) { sched_yield(); }
        }
        int status;
        waitpid(pid, &status, 0);
        long long elapsed = time_us() - start;
        printf("  Shared Memory: %lld us total, %lld us/iteration\n",
               elapsed, elapsed / PERF_ITERATIONS);

        munmap(shm, 8);
        shm_unlink("/perf_shm");
    }
}

static void perf_socket(void) {
    int sv[2];
    socketpair(AF_UNIX, SOCK_STREAM, 0, sv);

    pid_t pid = fork();
    if (pid == 0) {
        close(sv[1]);  /* child reads from sv[0] */
        char buf[8];
        for (int i = 0; i < PERF_ITERATIONS; i++) {
            read(sv[0], buf, 1);
        }
        close(sv[0]);
        exit(0);
    } else {
        close(sv[0]);  /* parent writes to sv[1] */
        long long start = time_us();
        for (int i = 0; i < PERF_ITERATIONS; i++) {
            write(sv[1], PERF_MSG, 1);
        }
        close(sv[1]);
        int status;
        waitpid(pid, &status, 0);
        long long elapsed = time_us() - start;
        printf("  Unix Socket:   %lld us total, %lld us/iteration\n",
               elapsed, elapsed / PERF_ITERATIONS);
    }
}

static void perf_compare(void) {
    printf("=== Performance Comparison ===\n");
    printf("  (%d iterations of 1-byte write+read)\n\n", PERF_ITERATIONS);

    perf_pipe();
    perf_fifo();
    perf_shm();
    perf_socket();

    printf("\n  (Shared memory is fastest because no kernel copy.)\n\n");
}

int main(void) {
    setvbuf(stdout, NULL, _IONBF, 0);  /* disable buffering for clean fork output */
    printf("IPC: Pipes, FIFOs, Shared Memory, Sockets\n");
    printf("===========================================\n\n");

    pipe_demo();
    fifo_demo();
    shm_demo();
    socket_demo();
    perf_compare();

    printf("Done.\n");
    return 0;
}
