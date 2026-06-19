# Sockets API — Build a TCP Echo Server

> Sockets API — Build a TCP Echo Server — the part of CS you can't skip.

**Type:** Learn
**Languages:** C, Rust
**Prerequisites:** Phase 09 lessons 01–09
**Time:** ~75 minutes

## Learning Objectives

- Understand the BSD sockets API and how it exposes the TCP/IP stack to user programs.
- Implement a working TCP echo server and client from scratch in C and Rust.
- Compare your implementation against production server architectures (nginx, Redis).
- Ship a reusable TCP echo server template for later network programming lessons.

## The Problem

You know that TCP provides reliable, ordered byte streams (Lesson 07). You know how IP routes packets (Lessons 03–05). But how does your application actually *send and receive bytes over the network*? The answer is the **sockets API** — the interface between user-space programs and the kernel's TCP/IP stack.

Without the sockets API, every piece of networked software — your browser, your database, your chat app — would need raw kernel-level access. The sockets API is the universal abstraction: one set of system calls works for TCP, UDP, Unix domain sockets, and more.

## The Concept

### What is a socket?

A **socket** is a kernel-managed endpoint for network communication. It is a file descriptor — you `read()` and `write()` on it just like a file. The kernel handles segmentation, retransmission, congestion control, and routing behind the scenes.

### The server lifecycle

```
socket()    →  create an endpoint
  ↓
bind()      →  attach to an address + port
  ↓
listen()    →  mark as passive (server) socket
  ↓
accept()    →  wait for a client connection
  ↓
read()/write()  ↔  exchange data
  ↓
close()     →  tear down the connection
```

### The client lifecycle

```
socket()    →  create an endpoint
  ↓
connect()   →  initiate TCP handshake (SYN)
  ↓
read()/write()  ↔  exchange data
  ↓
close()     →  tear down the connection
```

### Address structures

The sockets API uses generic `struct sockaddr` pointers, but you fill in protocol-specific structures:

**IPv4 — `struct sockaddr_in`:**
```c
struct sockaddr_in {
    sa_family_t    sin_family;  // AF_INET
    in_port_t      sin_port;    // port in NETWORK BYTE ORDER
    struct in_addr sin_addr;    // IPv4 address (4 bytes)
};
```

**IPv6 — `struct sockaddr_in6`:**
```c
struct sockaddr_in6 {
    sa_family_t     sin6_family;  // AF_INET6
    in_port_t       sin6_port;    // port in NETWORK BYTE ORDER
    uint32_t        sin6_flowinfo;
    struct in6_addr sin6_addr;    // IPv6 address (16 bytes)
    uint32_t        sin6_scope_id;
};
```

### Byte ordering

Network protocols use **big-endian** (network byte order). Your x86/ARM machine is usually little-endian. The conversion functions:

| Function | Direction |
|----------|-----------|
| `htons()` | host → network, 16-bit (port) |
| `ntohs()` | network → host, 16-bit |
| `htonl()` | host → network, 32-bit (address) |
| `ntohl()` | network → host, 32-bit |

Always convert ports and addresses. Forgetting this is the #1 beginner sockets bug.

### Blocking vs non-blocking

By default, `accept()`, `read()`, and `connect()` **block** — they suspend your process until data arrives. For handling multiple clients:

- **`fork()`** — one process per client (simple, wasteful)
- **`select()`/`poll()`** — monitor multiple sockets in one thread (event loop)
- **`epoll`/`kqueue`** — scalable I/O multiplexing (production servers)
- **`O_NONBLOCK`** — set on the file descriptor for non-blocking reads

### Error handling

Every sockets call can fail. Check return values and use `errno`:

```c
if (sockfd < 0) {
    perror("socket");
    // or: fprintf(stderr, "socket: %s\n", strerror(errno));
    exit(1);
}
```

## Build It

### Step 1: Minimal echo server (C)

A single-client echo server: accept one connection, echo everything back, then exit.

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <arpa/inet.h>

#define PORT 8080
#define BUF_SIZE 4096

int main(void) {
    int server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd < 0) { perror("socket"); exit(1); }

    int opt = 1;
    setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in addr = {
        .sin_family = AF_INET,
        .sin_port = htons(PORT),
        .sin_addr.s_addr = INADDR_ANY
    };

    if (bind(server_fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("bind"); exit(1);
    }
    if (listen(server_fd, 1) < 0) {
        perror("listen"); exit(1);
    }

    printf("Listening on port %d...\n", PORT);
    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);
    int client_fd = accept(server_fd, (struct sockaddr *)&client_addr, &client_len);
    if (client_fd < 0) { perror("accept"); exit(1); }

    char buf[BUF_SIZE];
    ssize_t n;
    while ((n = read(client_fd, buf, BUF_SIZE)) > 0) {
        write(client_fd, buf, n);  // echo back
    }

    close(client_fd);
    close(server_fd);
    return 0;
}
```

Compile and test:
```bash
gcc -o echo_server echo_server.c
./echo_server
# In another terminal: echo "hello" | nc localhost 8080
```

### Step 2: Multi-client echo server (C, fork())

The realistic version uses `fork()` to handle each client in a child process. The parent reaps zombie children with `SIGCHLD`.

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <signal.h>
#include <sys/wait.h>
#include <arpa/inet.h>

#define PORT 8080
#define BUF_SIZE 4096

void sigchld_handler(int sig) {
    (void)sig;
    while (waitpid(-1, NULL, WNOHANG) > 0);
}

void handle_client(int client_fd) {
    char buf[BUF_SIZE];
    ssize_t n;
    while ((n = read(client_fd, buf, BUF_SIZE)) > 0) {
        write(client_fd, buf, n);
    }
    close(client_fd);
}

int main(void) {
    struct sigaction sa = { .sa_handler = sigchld_handler };
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_RESTART;
    sigaction(SIGCHLD, &sa, NULL);

    int server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd < 0) { perror("socket"); exit(1); }

    int opt = 1;
    setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in addr = {
        .sin_family = AF_INET,
        .sin_port = htons(PORT),
        .sin_addr.s_addr = INADDR_ANY
    };

    if (bind(server_fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("bind"); exit(1);
    }
    if (listen(server_fd, 128) < 0) {
        perror("listen"); exit(1);
    }

    printf("Echo server on port %d...\n", PORT);

    while (1) {
        struct sockaddr_in client_addr;
        socklen_t client_len = sizeof(client_addr);
        int client_fd = accept(server_fd, (struct sockaddr *)&client_addr, &client_len);
        if (client_fd < 0) {
            if (errno == EINTR) continue;  // interrupted by signal
            perror("accept"); exit(1);
        }

        pid_t pid = fork();
        if (pid == 0) {
            close(server_fd);
            handle_client(client_fd);
            exit(0);
        }
        close(client_fd);
    }
}
```

### Step 3: Echo client (C)

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <arpa/inet.h>

#define PORT 8080
#define BUF_SIZE 4096

int main(int argc, char *argv[]) {
    const char *host = "127.0.0.1";
    if (argc > 1) host = argv[1];

    int sockfd = socket(AF_INET, SOCK_STREAM, 0);
    if (sockfd < 0) { perror("socket"); exit(1); }

    struct sockaddr_in server_addr = {
        .sin_family = AF_INET,
        .sin_port = htons(PORT),
    };
    inet_pton(AF_INET, host, &server_addr.sin_addr);

    if (connect(sockfd, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        perror("connect"); exit(1);
    }

    printf("Connected to %s:%d. Type messages (Ctrl-D to quit).\n", host, PORT);

    char buf[BUF_SIZE];
    while (fgets(buf, BUF_SIZE, stdin) != NULL) {
        write(sockfd, buf, strlen(buf));
        ssize_t n = read(sockfd, buf, BUF_SIZE);
        if (n <= 0) break;
        buf[n] = '\0';
        printf("echo: %s", buf);
    }

    close(sockfd);
    return 0;
}
```

### Step 4: Echo server (Rust, thread-per-connection)

```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

fn handle_client(mut stream: TcpStream) {
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if stream.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        std::process::exit(0);
    })
    .expect("failed to set signal handler");

    let listener = TcpListener::bind("0.0.0.0:8080")?;
    println!("Echo server listening on port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| handle_client(stream));
            }
            Err(e) => eprintln!("accept failed: {}", e),
        }
    }
    Ok(())
}
```

For the Rust version without the `ctrlc` crate, use:
```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_client(mut stream: TcpStream) {
    let addr = stream.peer_addr().unwrap();
    println!("Client connected: {}", addr);
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => {
                println!("Client disconnected: {}", addr);
                break;
            }
            Ok(n) => {
                if stream.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
            Err(e) => {
                eprintln!("read error from {}: {}", addr, e);
                break;
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    println!("Echo server listening on port 8080");

    let mut handles = Vec::new();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handles.push(thread::spawn(|| handle_client(stream)));
            }
            Err(e) => eprintln!("accept: {}", e),
        }
    }
    Ok(())
}
```

## Use It

**Production servers do more:**

- **nginx**: uses `epoll` (Linux) / `kqueue` (BSD/macOS) for thousands of concurrent connections in one process — no `fork()`, no threads. Master process forks worker processes, each running an event loop.
- **Redis**: single-threaded event loop using `epoll`. No fork for client handling. Commands are processed sequentially — simple and fast because operations are in-memory.
- **Go net/http**: goroutine-per-connection. The Go runtime multiplexes goroutines onto OS threads with an internal poller.

Your fork-based echo server is correct but doesn't scale: 10,000 clients = 10,000 processes. Production systems use event loops or async I/O.

## Read the Source

- Linux kernel `net/socket.c` — the actual syscalls (`sys_socket`, `sys_bind`, `sys_connect`, `sys_accept`) that the C library wraps.
- glibc `sysdeps/unix/sysv/linux/socket.c` — how glibc's `socket()` calls `socketcall` or direct syscalls.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained TCP echo server + client template you can reuse in later phases (Lesson 15: WebSockets, Lesson 22: HTTP/2 server).**

## Exercises

1. **Easy** — Reproduce the minimal echo server (Step 1) in C without looking at the lesson code. Compile and test with `nc localhost 8080`.

2. **Medium** — Add `select()` multiplexing: handle multiple clients in a single process without `fork()`. Use `FD_SET` / `FD_ZERO` to monitor the server socket and all client sockets. Close idle connections after 60 seconds of inactivity using `select()`'s timeout parameter.

3. **Hard** — Implement a chat server: when one client sends a message, broadcast it to all other connected clients. Use `epoll` (Linux) or `kqueue` (macOS) for efficient I/O multiplexing. Handle client disconnections gracefully and broadcast "user left" notifications.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Socket | "Open a socket" | A kernel file descriptor representing one end of a network connection |
| `bind()` | "Bind to a port" | Associate a socket with a local IP:port address |
| `listen()` | "Start listening" | Mark a socket as passive (server) — the kernel queues incoming SYN packets |
| `accept()` | "Accept a connection" | Dequeue a completed connection from the listen queue, return a new connected socket FD |
| `connect()` | "Connect to the server" | Initiate a TCP 3-way handshake to a remote address |
| Network byte order | "NBO" | Big-endian byte ordering used in all network protocols |
| `SO_REUSEADDR` | "Set reuse addr" | Socket option allowing bind to a port in TIME_WAIT state |

## Further Reading

- W. Richard Stevens, *Unix Network Programming, Volume 1* — the definitive sockets reference.
- [Beej's Guide to Network Programming](https://beej.us/guide/bgnet/) — free, beginner-friendly sockets tutorial.
- Linux `man 7 socket` — complete reference for socket options, address families, and protocols.
