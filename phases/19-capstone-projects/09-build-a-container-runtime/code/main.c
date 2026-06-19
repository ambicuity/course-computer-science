// Build a Container Runtime — Namespace Setup (C)
// Run: gcc -o nsenter main.c && ./nsenter <rootfs> <command> [args...]
// Note: Linux only — requires clone(), _GNU_SOURCE, sched.h
//
// Uses clone() with CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWUTS to create
// isolated namespaces, mount procfs, set hostname, and exec the container command.

#define _GNU_SOURCE
#include <sched.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/mount.h>
#include <sys/wait.h>
#include <signal.h>
#include <errno.h>
#include <string.h>

// Set up a minimal container environment
int container_child(void *arg) {
    char **argv = (char **)arg;

    // Set hostname
    sethostname("container", 9);

    // Mount proc filesystem (requires PID namespace)
    if (mount("proc", "/proc", "proc", 0, NULL) == -1) {
        fprintf(stderr, "mount proc: %s\n", strerror(errno));
    }

    // Set environment
    setenv("container", "true", 1);

    // Execute the container command
    execvp(argv[0], argv);
    perror("execvp failed");
    return 1;
}

#define STACK_SIZE (1024 * 1024) // 1MB child stack

int main(int argc, char *argv[]) {
    if (argc < 3) {
        fprintf(stderr, "Usage: %s <rootfs> <command> [args...]\n", argv[0]);
        fprintf(stderr, "\nExample: %s / /bin/sh -c 'echo hello from container'\n", argv[0]);
        return 1;
    }

    char *rootfs = argv[1];
    char **cmd = &argv[2];

    // Allocate stack for child
    char *stack = malloc(STACK_SIZE);
    if (!stack) { perror("malloc"); return 1; }

    // Clone with new PID and mount namespaces
    int flags = CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWUTS | SIGCHLD;

    pid_t child = clone(container_child, stack + STACK_SIZE, flags, cmd);
    if (child == -1) {
        perror("clone");
        free(stack);
        return 1;
    }

    printf("Container started with PID %d (host), PID 1 (container)\n", child);

    // Wait for container to exit
    int status;
    waitpid(child, &status, 0);

    if (WIFEXITED(status)) {
        printf("Container exited with status %d\n", WEXITSTATUS(status));
    } else if (WIFSIGNALED(status)) {
        printf("Container killed by signal %d\n", WTERMSIG(status));
    }

    free(stack);
    return 0;
}
