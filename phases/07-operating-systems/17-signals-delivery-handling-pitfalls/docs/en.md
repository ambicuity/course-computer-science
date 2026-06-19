# Lesson 17: Signals — Delivery, Handling, Pitfalls

## Why This Matters

Signals are the kernel's way of notifying a process that something happened — a timer expired, a child died, the user pressed Ctrl+C, or the process accessed invalid memory. Every systems programmer must understand signals: how they are delivered, how to handle them, and the subtle bugs that arise when handlers interact with normal program execution.

## What Is a Signal?

A signal is a **software interrupt**. The kernel delivers it by interrupting the process's normal execution and invoking a handler function (or taking a default action).

```
  Kernel event               Signal           Process
 ┌──────────┐  ─────────►  ┌────────┐  ───►  ┌─────────────┐
 │ user hits │              │ SIGINT │        │ run handler │
 │ Ctrl+C    │              └────────┘        │ or default  │
 └──────────┘                                 └─────────────┘
```

## Common Signals

| Signal | Number | Default Action | Cause |
|--------|--------|---------------|-------|
| `SIGINT` | 2 | Terminate | User pressed Ctrl+C |
| `SIGSEGV` | 11 | Core dump | Segmentation fault (invalid memory access) |
| `SIGTERM` | 15 | Terminate | Polite kill request (`kill <pid>`) |
| `SIGKILL` | 9 | Terminate | Force kill (cannot be caught or ignored) |
| `SIGPIPE` | 13 | Terminate | Write to a pipe/socket with no reader |
| `SIGCHLD` | 17 | Ignore | Child process stopped or terminated |
| `SIGALRM` | 14 | Terminate | Timer expired (`alarm()`) |
| `SIGSTOP` | 19 | Stop | Pause process (cannot be caught) |
| `SIGUSR1` | 10 | Terminate | User-defined signal 1 |
| `SIGUSR2` | 12 | Terminate | User-defined signal 2 |

## Signal Handlers

### `signal()` — Simple Interface

```c
#include <signal.h>

void handler(int sig) {
    printf("Caught signal %d\n", sig);
}

signal(SIGINT, handler);  /* install handler */
```

`signal()` is simple but has portability issues — on some systems the handler resets to `SIG_DFL` after the first invocation.

### `sigaction()` — Recommended Interface

```c
struct sigaction sa;
sa.sa_handler = handler;
sigemptyset(&sa.sa_mask);
sa.sa_flags = SA_RESTART;  /* restart interrupted syscalls */

sigaction(SIGINT, &sa, NULL);
```

**Advantages of `sigaction()`:**
- Handler stays installed after invocation (no `SIG_DFL` reset)
- `SA_RESTART` — automatically restarts slow syscalls interrupted by signals
- `SA_SIGINFO` — receive extended signal info (address of fault, sender PID)
- Fine-grained control over signal mask during handler execution

## Signal Mask — Blocking Signals

You can **block** signals during critical sections. Blocked signals are **pending** — delivered when unblocked.

```c
sigset_t mask;
sigemptyset(&mask);
sigaddset(&mask, SIGINT);

/* Block SIGINT */
sigprocmask(SIG_BLOCK, &mask, NULL);

/* Critical section — SIGINT won't interrupt here */

/* Unblock — pending SIGINT is delivered now */
sigprocmask(SIG_UNBLOCK, &mask, NULL);
```

## `sigsetjmp` / `siglongjmp` — Non-Local Goto with Signal Mask

The standard `setjmp`/`longjmp` do not restore the signal mask. `sigsetjmp`/`siglongjmp` do:

```c
sigjmp_buf env;

void handler(int sig) {
    siglongjmp(env, 1);  /* jumps back, restoring signal mask */
}

if (sigsetjmp(env, 1) == 0) {
    /* Normal path — set up handler, do risky work */
} else {
    /* Arrived here via siglongjmp from handler */
}
```

This is used in interpreters and servers to recover from faults without terminating.

## Pitfalls

### 1. Only Async-Signal-Safe Functions in Handlers

A signal handler can interrupt code at **any point** — even in the middle of `malloc()` or `printf()`. Calling non-reentrant functions from a handler causes undefined behavior.

**Safe functions** (from `man 7 signal-safety`): `write()`, `_exit()`, `signal()`, `sigprocmask()`, and a small fixed set.

**UNSAFE:** `printf()`, `malloc()`, `free()`, `strcpy()`, any stdio function.

```c
/* WRONG */
void bad_handler(int sig) {
    printf("caught signal %d\n", sig);  /* printf is NOT async-signal-safe */
    char *p = malloc(100);              /* malloc is NOT async-signal-safe */
}

/* RIGHT */
void good_handler(int sig) {
    const char msg[] = "caught signal\n";
    write(STDOUT_FILENO, msg, sizeof(msg) - 1);  /* write() IS safe */
    got_signal = 1;  /* volatile sig_atomic_t variable */
}
```

### 2. Use `volatile sig_atomic_t` for Shared State

The handler and main code share a flag variable. It must be:
- `volatile` — prevents compiler from caching it in a register
- `sig_atomic_t` — guarantees atomic read/write (fits in one word)

```c
volatile sig_atomic_t got_signal = 0;
```

### 3. Signal Handler Races

If the handler needs to modify shared data structures, block the signal during modifications in the main code. Otherwise the handler can interrupt the modification and see inconsistent state.

### 4. EINTR on Slow System Calls

Without `SA_RESTART`, signals interrupt blocking calls like `read()`, `accept()`, `select()`. These return `-1` with `errno == EINTR` and must be retried:

```c
ssize_t n;
do {
    n = read(fd, buf, sizeof(buf));
} while (n < 0 && errno == EINTR);
```

## Build It

See `code/main.c` for complete working demos:

- `sigint_handler()` — handle Ctrl+C gracefully, print message and continue
- `sigsegv_handler()` — catch segmentation fault, print the faulting address
- `sigchld_handler()` — automatically reap child processes
- `alarm_demo()` — SIGALRM timeout for a blocking operation
- `signal_mask_demo()` — block/unblock signals during a critical section
- `pitfall_demo()` — demonstrates what NOT to do (calling printf/malloc in a handler)

## Use It

Signals in real systems:

- **Ctrl+C (SIGINT):** Your terminal sends SIGINT to the foreground process group when you press Ctrl+C
- **Shell job control:** `SIGTSTP` (Ctrl+Z) stops a process; `SIGCONT` resumes it
- **Daemons:** SIGHUP causes many daemons to reload configuration
- **`kill -9`:** SIGKILL cannot be caught — the kernel unconditionally terminates the process
- **GDB:** The debugger uses `SIGTRAP` (breakpoint) and `SIGSTOP` to control traced processes

## Ship It

The signal handling toolkit in `code/main.c` provides reusable patterns for safe signal handling in systems software.

## Exercises

### Level 1 — Recall

Why can't you call `printf()` in a signal handler? What is the difference between `signal()` and `sigaction()`? What does `volatile sig_atomic_t` guarantee?

### Level 2 — Application

Write a program that uses SIGALRM to implement a 5-second timeout on a `read()` from stdin. If the user doesn't type anything within 5 seconds, print "Timed out!" and exit. Use `sigaction()` with `SA_RESTART` and handle the race condition correctly.

### Level 3 — Build

Implement a process supervisor: a parent process that forks a child, monitors it with SIGCHLD, and automatically restarts it if it dies. The parent should handle SIGINT to cleanly shut down (kill the child, wait for it, and exit). Use `sigprocmask()` to block SIGCHLD while modifying shared state. Support a command-line argument for the child program to run.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Signal | "Software interrupt" | Kernel notification that interrupts process execution |
| Signal handler | "Callback for signals" | Function invoked when a signal is delivered |
| Signal mask | "Blocked signals" | Set of signals whose delivery is deferred until unblocked |
| Async-signal-safe | "Safe to call in a handler" | Function that can be safely called from a signal handler (reentrant or doesn't modify shared state) |
| sig_atomic_t | "Atomic variable for signals" | Integer type guaranteed to be read/written atomically |

## Further Reading

- `man 7 signal` — Linux signal overview
- `man 7 signal-safety` — async-signal-safe functions list
- Stevens, *Advanced Programming in the UNIX Environment*, Chapter 10
- Linux kernel source: `kernel/signal.c`
