# Lesson 05: Processes — fork, exec, wait

## Why This Matters

When you type `ls` in a terminal, the shell creates a new process, loads the `ls` program into it, and waits for it to finish. This fork-exec-wait pattern is how *every* program runs on Unix. Understanding processes is understanding how the OS runs multiple programs simultaneously.

## What Is a Process?

A process is a **program in execution**. The same program (`/bin/ls`) can have multiple processes running simultaneously, each with its own state.

### Process Control Block (PCB)

The kernel maintains a **PCB** for each process — the complete record of that process's state:

```
┌────────────────────────────────────┐
│         Process Control Block      │
├────────────────────────────────────┤
│  PID          Process ID (unique)  │
│  State        running / ready /    │
│               blocked / zombie     │
│  Registers    saved CPU state      │
│  PC           program counter      │
│  SP           stack pointer        │
│  Memory Map   page table entries   │
│  File Desctbl open files (stdin,   │
│               stdout, etc.)        │
│  Parent PID   who created me       │
│  Children     PIDs I created       │
│  UID / GID    user and group IDs   │
│  Nice value   scheduling priority  │
└────────────────────────────────────┘
```

### Process States

```
           fork()
    ┌──────────────► READY
    │                 │
    │  scheduler      │ scheduler picks
    │  deschedules     ▼
 NEW ──► RUNNING ◄─────┘
         │
         ├──► BLOCKED  (waiting for I/O, timer, signal)
         │        │
         │        │ event occurs
         │        ▼
         │       READY
         │
         └──► EXITED (zombie until parent calls wait)
```

## fork()

`fork()` creates a new process by duplicating the calling process.

```c
pid_t pid = fork();
```

**Return value:**
- In the **child**: returns `0`
- In the **parent**: returns the child's PID
- On error: returns `-1`

After `fork()`, parent and child are **identical copies** — same code, same variables, same open files — but they run independently.

```c
#include <stdio.h>
#include <unistd.h>

int main(void) {
    int x = 42;
    pid_t pid = fork();

    if (pid == 0) {
        printf("Child: x=%d, my PID=%d\n", x, getpid());
        x = 100;
        printf("Child: x is now %d\n", x);
    } else {
        printf("Parent: child PID=%d, my PID=%d\n", pid, getpid());
        printf("Parent: x is still %d\n", x); /* x is still 42 */
    }
    return 0;
}
```

### Copy-on-Write (COW)

Copying all memory would be wasteful. Modern kernels use **copy-on-write**: parent and child share the same physical pages initially. Only when one writes to a page does the kernel make a private copy. This makes `fork()` fast — O(1) in many cases, no actual copying until needed.

## exec()

`exec()` **replaces** the current process image with a new program. It does not create a new process.

```c
execl("/bin/ls", "ls", "-l", NULL);   /* list form */
execv("/bin/ls", argv);                 /* vector form */
```

The process PID stays the same. The old code, data, and stack are replaced. Only the PID, open file descriptors, and signal handlers are preserved.

## wait()

`wait()` blocks the calling process until a child exits.

```c
int status;
pid_t child = wait(&status);  /* wait for any child */
```

`waitpid()` waits for a specific child:

```c
pid_t child = waitpid(pid, &status, 0);
```

`WEXITSTATUS(status)` extracts the child's exit code.

## Zombie Processes

When a child exits, it becomes a **zombie**: its PCB remains because the parent might want to check its exit status. If the parent never calls `wait()`, the zombie stays forever, consuming a PID slot.

```
Child calls exit() → state = ZOMBIE → parent calls wait() → PCB freed
```

## Orphan Processes

If the parent exits before the child, the child becomes an **orphan** and is adopted by `init` (PID 1), which calls `wait()` to clean it up.

## Process Tree

```
        init (PID 1)
           │
      ┌────┴────┐
      │         │
    bash      sshd
      │
   ┌──┴──┐
   │     │
   ls    gcc
```

Every process (except `init`) has exactly one parent.

## Build It

We'll write C programs demonstrating fork, exec, wait, zombies, and inter-process communication via pipes.

## Use It

The shell is the canonical example:

```bash
ls -l /tmp
```

1. Shell calls `fork()` → child process created
2. Child calls `exec("/bin/ls", "-l", "/tmp")` → replaces itself with `ls`
3. Shell calls `wait()` → blocks until `ls` finishes
4. Shell prints next prompt

## Ship It

See `code/main.c` for complete working demos of each concept.

## Exercises

### Level 1 — Recall

What does `fork()` return in the child process? What does it return in the parent? What happens to variables after a `fork()`?

### Level 2 — Application

Write a program that forks 3 children. Each child should print its PID and sleep for a different duration (1, 2, 3 seconds). The parent should wait for all children and print each child's PID and exit status as they complete.

### Level 3 — Build

Implement a mini shell that:
1. Reads a command line from stdin
2. Parses it into a program and arguments
3. Forks a child to execute the command
4. Waits for the child and prints its exit status
5. Repeats until the user types "exit"

Handle `cd` as a built-in (use `chdir()` in the parent process, since `exec` would lose the directory change).
