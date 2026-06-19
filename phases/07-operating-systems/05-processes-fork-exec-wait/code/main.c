#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>
#include <errno.h>

/*
 * Process management demos: fork, exec, wait, zombie, pipe.
 * Compile: gcc -o proc_demo main.c
 * Run:     ./proc_demo
 */

/* ---------- fork_demo ---------- */
static void fork_demo(void) {
    printf("=== fork() Demo ===\n\n");

    int x = 42;
    printf("Before fork: x = %d, PID = %d\n\n", x, getpid());

    pid_t pid = fork();

    if (pid < 0) {
        perror("fork");
        exit(1);
    }

    if (pid == 0) {
        /* Child process */
        printf("CHILD:  my PID = %d, parent PID = %d\n", getpid(), getppid());
        printf("CHILD:  x = %d (copy of parent's x)\n", x);
        x = 100;
        printf("CHILD:  x is now %d (my copy only)\n", x);
        printf("CHILD:  exiting with code 42\n");
        exit(42);
    } else {
        /* Parent process */
        printf("PARENT: my PID = %d, child PID = %d\n", getpid(), pid);
        printf("PARENT: x = %d (unchanged)\n", x);
        printf("PARENT: waiting for child...\n");

        int status;
        waitpid(pid, &status, 0);

        if (WIFEXITED(status)) {
            printf("PARENT: child exited with status %d\n", WEXITSTATUS(status));
        }
    }
    printf("\n");
}

/* ---------- exec_demo ---------- */
static void exec_demo(void) {
    printf("=== fork() + exec() Demo ===\n\n");

    pid_t pid = fork();

    if (pid < 0) {
        perror("fork");
        exit(1);
    }

    if (pid == 0) {
        /* Child: exec ls */
        printf("CHILD:  exec'ing /bin/ls -l\n\n");
        execl("/bin/ls", "ls", "-l", "/tmp", NULL);

        /* exec only returns on error */
        perror("execl");
        exit(1);
    } else {
        /* Parent waits */
        int status;
        waitpid(pid, &status, 0);

        if (WIFEXITED(status)) {
            printf("\nPARENT: ls exited with status %d\n\n", WEXITSTATUS(status));
        }
    }
}

/* ---------- wait_demo ---------- */
static void wait_demo(void) {
    printf("=== wait() with Multiple Children ===\n\n");

    int num_children = 3;
    pid_t children[3];

    for (int i = 0; i < num_children; i++) {
        pid_t pid = fork();

        if (pid < 0) {
            perror("fork");
            exit(1);
        }

        if (pid == 0) {
            /* Child: sleep for different durations */
            int sleep_time = i + 1;
            printf("CHILD %d: PID %d, sleeping %d sec\n", i, getpid(), sleep_time);
            sleep(sleep_time);
            printf("CHILD %d: PID %d, exiting\n", i, getpid());
            exit(10 + i);
        } else {
            children[i] = pid;
        }
    }

    /* Parent waits for all children in order */
    printf("\nPARENT: waiting for %d children...\n\n", num_children);

    for (int i = 0; i < num_children; i++) {
        int status;
        pid_t finished = wait(&status); /* wait for any child */

        if (WIFEXITED(status)) {
            printf("PARENT: child PID %d exited with status %d\n",
                   finished, WEXITSTATUS(status));
        }
    }
    printf("\n");
}

/* ---------- zombie_demo ---------- */
static void zombie_demo(void) {
    printf("=== Zombie Process Demo ===\n\n");

    pid_t pid = fork();

    if (pid < 0) {
        perror("fork");
        exit(1);
    }

    if (pid == 0) {
        /* Child exits immediately */
        printf("CHILD:  PID %d exiting now (becomes zombie)\n", getpid());
        exit(0);
    } else {
        /* Parent sleeps without calling wait() */
        printf("PARENT: child PID %d, sleeping 2 sec without calling wait()\n", pid);
        printf("PARENT: during this time, child is a zombie\n");
        printf("PARENT: check: ps aux | grep defunct\n");

        /* Show zombie in /proc (if on Linux) or just sleep */
        sleep(2);

        printf("PARENT: now calling wait() to reap zombie\n");
        int status;
        waitpid(pid, &status, 0);
        printf("PARENT: zombie reaped, PCB freed\n\n");
    }
}

/* ---------- pipe_demo ---------- */
static void pipe_demo(void) {
    printf("=== Pipe Communication Demo ===\n\n");

    int pipefd[2]; /* pipefd[0] = read end, pipefd[1] = write end */

    if (pipe(pipefd) < 0) {
        perror("pipe");
        exit(1);
    }

    pid_t pid = fork();

    if (pid < 0) {
        perror("fork");
        exit(1);
    }

    if (pid == 0) {
        /* Child: read from pipe */
        close(pipefd[1]); /* close write end */

        char buf[256];
        ssize_t n = read(pipefd[0], buf, sizeof(buf) - 1);
        if (n > 0) {
            buf[n] = '\0';
            printf("CHILD:  received '%s' from parent via pipe\n", buf);
        }

        close(pipefd[0]);
        exit(0);
    } else {
        /* Parent: write to pipe */
        close(pipefd[0]); /* close read end */

        const char *msg = "Hello from parent!";
        printf("PARENT: sending '%s' to child via pipe\n", msg);
        write(pipefd[1], msg, strlen(msg));

        close(pipefd[1]);

        int status;
        waitpid(pid, &status, 0);
        printf("PARENT: child done\n\n");
    }
}

int main(void) {
    printf("Processes: fork, exec, wait\n");
    printf("===========================\n\n");

    fork_demo();
    exec_demo();
    wait_demo();
    zombie_demo();
    pipe_demo();

    printf("Done.\n");
    return 0;
}
