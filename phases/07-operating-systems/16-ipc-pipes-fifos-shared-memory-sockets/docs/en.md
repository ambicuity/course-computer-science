# Lesson 16: IPC — Pipes, FIFOs, Shared Memory, Sockets

## Why This Matters

Processes are isolated — they have separate address spaces and can't see each other's memory. Yet real systems need processes to cooperate: a web server hands connections to worker processes, a database shares a buffer pool across backends, a shell pipes `ls` output into `grep`. **Inter-Process Communication (IPC)** is the set of mechanisms the kernel provides for processes to exchange data and synchronize.

## IPC Mechanisms Overview

```
                    Speed   Complexity   Scope              Direction
                 ┌──────────────────────────────────────────────────────┐
  Pipe           │  fast   │  simple  │ parent-child only │ unidirectional │
  FIFO (named)   │  fast   │  simple  │ any process       │ unidirectional │
  Shared Memory  │ fastest │  medium  │ any process       │ bidirectional  │
  Message Queue  │  fast   │  medium  │ any process       │ unidirectional │
  Unix Socket    │  fast   │  higher  │ same host         │ bidirectional  │
  Network Socket │ slower  │  higher  │ any host          │ bidirectional  │
                 └──────────────────────────────────────────────────────┘
```

## 1. Pipes (Unnamed)

A pipe is a kernel-managed byte buffer with two file descriptors: one for reading, one for writing. Created with `pipe()`. Data written to `fd[1]` can be read from `fd[0]`.

```c
int fd[2];
pipe(fd);          /* fd[0] = read end, fd[1] = write end */

/* Parent writes, child reads */
pid_t pid = fork();
if (pid == 0) {
    close(fd[1]);
    char buf[256];
    ssize_t n = read(fd[0], buf, sizeof(buf));
    /* ... */
    close(fd[0]);
} else {
    close(fd[0]);
    write(fd[1], "hello", 5);
    close(fd[1]);
}
```

**Constraints:** Unidirectional. Parent-child only (or the process must inherit the fd via fork). The pipe has a fixed buffer size (typically 64 KB on Linux). If the buffer is full, `write()` blocks. If empty, `read()` blocks.

**Shell pipes** (`ls | grep foo`) use unnamed pipes — the shell forks two children, connects the write end of one pipe to child 1's stdout and the read end to child 2's stdin.

## 2. FIFOs (Named Pipes)

A FIFO is a pipe with a name in the filesystem, created with `mkfifo()`. Any process with permission can open it.

```c
mkfifo("/tmp/myfifo", 0666);

/* Writer */
int wfd = open("/tmp/myfifo", O_WRONLY);
write(wfd, "data", 4);
close(wfd);

/* Reader */
int rfd = open("/tmp/myfifo", O_RDONLY);
char buf[256];
ssize_t n = read(rfd, buf, sizeof(buf));
close(rfd);
```

The `open()` blocks until both ends are connected (unless `O_NONBLOCK` is set). The FIFO persists in the filesystem until you `unlink()` it.

## 3. Shared Memory

Shared memory is the **fastest** IPC mechanism — the kernel maps the same physical page into multiple processes' address spaces. No copying. But you must handle synchronization yourself (typically with semaphores).

```c
#include <sys/shm.h>
#include <semaphore.h>

/* Create shared memory segment */
int shmid = shmget(IPC_PRIVATE, 4096, IPC_CREAT | 0666);
char *shm = shmat(shmid, NULL, 0);  /* attach to address space */

/* Synchronization via named semaphore */
sem_t *sem = sem_open("/mysem", O_CREAT, 0666, 1);

sem_wait(sem);
strcpy(shm, "shared data");
sem_post(sem);

/* Cleanup */
shmdt(shm);          /* detach */
shmctl(shmid, IPC_RMID, NULL); /* remove segment */
```

**Why shared memory needs synchronization:** Both processes see the same memory. Without a mutex or semaphore, concurrent writes produce data races — exactly like threads, but across process boundaries.

## 4. Message Queues

Message queues pass discrete messages (not a raw byte stream). Each message has a type and priority.

```c
#include <sys/msg.h>

struct msgbuf {
    long mtype;          /* message type (must be > 0) */
    char mtext[256];     /* message data */
};

/* Create queue */
int msqid = msgget(IPC_PRIVATE, IPC_CREAT | 0666);

/* Send */
struct msgbuf msg;
msg.mtype = 1;
strcpy(msg.mtext, "hello queue");
msgsnd(msqid, &msg, sizeof(msg.mtext), 0);

/* Receive by type */
msgrcv(msqid, &msg, sizeof(msg.mtext), 1, 0);

/* Cleanup */
msgctl(msqid, IPC_RMID, NULL);
```

## 5. Unix Domain Sockets

Unix domain sockets are like network sockets but for same-host communication. They support bidirectional data flow and can even pass file descriptors between processes via `SCM_RIGHTS`.

```c
#include <sys/socket.h>
#include <sys/un.h>

int sv[2]; /* sv[0] = server fd, sv[1] = client fd */
socketpair(AF_UNIX, SOCK_STREAM, 0, sv);

/* Parent uses sv[0], child uses sv[1] */
pid_t pid = fork();
if (pid == 0) {
    close(sv[0]);
    write(sv[1], "from child", 10);
    close(sv[1]);
} else {
    close(sv[1]);
    char buf[256];
    read(sv[0], buf, sizeof(buf));
    close(sv[0]);
}
```

## 6. Network Sockets

Network sockets (`AF_INET`, `AF_INET6`) extend IPC across hosts via TCP or UDP. They are the foundation of all networked applications.

```c
int sock = socket(AF_INET, SOCK_STREAM, 0);
struct sockaddr_in addr = { .sin_family = AF_INET,
                             .sin_port = htons(8080),
                             .sin_addr.s_addr = INADDR_ANY };
bind(sock, (struct sockaddr *)&addr, sizeof(addr));
listen(sock, 5);
int client = accept(sock, NULL, NULL);
```

## Build It

See `code/main.c` for complete working demos:

- `pipe_demo()` — parent sends a message to child via an unnamed pipe
- `fifo_demo()` — one process writes to `/tmp/demo_fifo`, another reads
- `shm_demo()` — two processes share memory with semaphore synchronization
- `socket_demo()` — Unix domain socket pair, bidirectional communication

## Use It

Real-world IPC usage:

- **Shell pipes** (`|`) — unnamed pipes created by the shell between pipeline stages
- **Database shared memory** — PostgreSQL uses shared memory for its buffer pool and shared buffers
- **SystemD sockets** — Unix domain sockets activate services on demand
- **`/dev/log`** — syslog uses a Unix domain socket for all local log messages
- **D-Bus** — Linux desktop IPC uses Unix domain sockets with a message bus protocol

## Ship It

The IPC demo collection in `code/main.c` is a reusable reference for choosing and implementing the right IPC mechanism for any given task.

## Exercises

### Level 1 — Recall

What are the key differences between pipes and FIFOs? When would you choose shared memory over pipes? Why does shared memory require explicit synchronization while pipes do not?

### Level 2 — Application

Write a program that creates a shared memory segment, then forks a child. The parent writes 10 integers into shared memory. The child reads them, computes their sum, and writes the result back to shared memory. Use a semaphore to synchronize access. The parent reads and prints the sum.

### Level 3 — Build

Implement a simple chat system using Unix domain sockets. A server process listens on a socket, accepts connections from multiple clients (using `select()` or `poll()`), and broadcasts each message to all connected clients. Handle client disconnects gracefully.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Pipe | "A way to connect processes" | Kernel byte buffer with two fds; unidirectional; parent-child only |
| FIFO | "Named pipe" | A pipe with a filesystem entry; any process can open it |
| Shared Memory | "Fastest IPC" | Same physical page mapped into multiple address spaces; needs synchronization |
| Message Queue | "Structured IPC" | Kernel-managed queue of typed messages with priority ordering |
| Unix Socket | "Local IPC" | Bidirectional IPC channel on the same host; can pass fds |

## Further Reading

- Stevens, *UNIX Network Programming, Volume 2: Interprocess Communications*
- `man 7 pipe`, `man 7 fifo`, `man 7 shm_overview`, `man 7 unix`
- Linux kernel source: `ipc/pipe.c`, `ipc/shm.c`, `net/unix/af_unix.c`
