#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <signal.h>
#include <sys/wait.h>
#include <errno.h>
#include <setjmp.h>

/*
 * Signal handling demos: SIGINT, SIGSEGV, SIGCHLD, SIGALRM,
 * signal mask, and pitfalls.
 * Compile: gcc -o signal_demo main.c
 * Run:     ./signal_demo
 */

/* ---------- sigint_handler ---------- */
static volatile sig_atomic_t sigint_count = 0;

static void sigint_handler(int sig) {
    (void)sig;
    sigint_count++;
    /* Use write() — it's async-signal-safe. printf() is NOT. */
    const char msg[] = "\nCaught SIGINT (Ctrl+C). Will exit after 3 catches.\n";
    write(STDOUT_FILENO, msg, sizeof(msg) - 1);
}

static void sigint_demo(void) {
    printf("=== SIGINT Handler Demo ===\n");
    printf("  Press Ctrl+C to trigger (or it will auto-trigger)\n\n");

    struct sigaction sa;
    sa.sa_handler = sigint_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = 0;
    sigaction(SIGINT, &sa, NULL);

    /* Auto-trigger for demo purposes */
    kill(getpid(), SIGINT);

    if (sigint_count >= 3) {
        printf("  Caught %d SIGINTs, stopping.\n\n", sigint_count);
    }

    /* Restore default */
    signal(SIGINT, SIG_DFL);
}

/* ---------- sigsegv_handler ---------- */
static void sigsegv_handler(int sig, siginfo_t *info, void *context) {
    (void)sig;
    (void)context;
    char msg[128];
    int len = snprintf(msg, sizeof(msg),
        "Caught SIGSEGV! Faulting address: %p\n",
        info->si_addr);
    write(STDOUT_FILENO, msg, len);

    /* Use _exit() — it's async-signal-safe. exit() is NOT. */
    _exit(1);
}

static void sigsegv_demo(void) {
    printf("=== SIGSEGV Handler Demo ===\n\n");

    struct sigaction sa;
    sa.sa_sigaction = sigsegv_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_SIGINFO;
    sigaction(SIGSEGV, &sa, NULL);

    pid_t pid = fork();
    if (pid == 0) {
        /* Child: trigger segfault */
        printf("CHILD:  about to dereference NULL pointer...\n");
        volatile int *p = NULL;
        (void)*p;  /* segfault */
        exit(0);   /* never reached */
    } else {
        int status;
        waitpid(pid, &status, 0);
        if (WIFSIGNALED(status)) {
            printf("PARENT: child killed by signal %d\n\n", WTERMSIG(status));
        }
    }

    signal(SIGSEGV, SIG_DFL);
}

/* ---------- sigchld_handler ---------- */
static void sigchld_handler(int sig) {
    (void)sig;
    /* Reap all finished children */
    int saved_errno = errno;  /* preserve errno */
    while (waitpid(-1, NULL, WNOHANG) > 0) {}
    errno = saved_errno;
}

static void sigchld_demo(void) {
    printf("=== SIGCHLD Handler Demo ===\n\n");

    struct sigaction sa;
    sa.sa_handler = sigchld_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_RESTART | SA_NOCLDSTOP;
    sigaction(SIGCHLD, &sa, NULL);

    printf("PARENT: forking 3 children...\n");

    for (int i = 0; i < 3; i++) {
        pid_t pid = fork();
        if (pid == 0) {
            printf("CHILD %d: PID %d, sleeping %d sec\n", i, getpid(), i + 1);
            sleep(i + 1);
            printf("CHILD %d: exiting\n", i);
            _exit(0);
        }
    }

    /* Parent waits a bit for children to finish */
    sleep(5);
    printf("PARENT: all children reaped automatically by SIGCHLD handler\n\n");

    signal(SIGCHLD, SIG_DFL);
}

/* ---------- alarm_demo ---------- */
static void alarm_handler(int sig) {
    (void)sig;
    const char msg[] = "SIGALRM received! Timeout!\n";
    write(STDOUT_FILENO, msg, sizeof(msg) - 1);
}

static void alarm_demo(void) {
    printf("=== SIGALRM Timeout Demo ===\n\n");

    struct sigaction sa;
    sa.sa_handler = alarm_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = 0;
    sigaction(SIGALRM, &sa, NULL);

    printf("Setting alarm for 2 seconds...\n");
    alarm(2);

    /* Sleep simulates blocking work that takes too long */
    sleep(5);

    printf("(Program continues after alarm)\n\n");

    alarm(0);  /* cancel any pending alarm */
    signal(SIGALRM, SIG_DFL);
}

/* ---------- signal_mask_demo ---------- */
static void signal_mask_demo(void) {
    printf("=== Signal Mask Demo ===\n\n");

    sigset_t mask, oldmask;
    sigemptyset(&mask);
    sigaddset(&mask, SIGINT);

    printf("Blocking SIGINT for 2 seconds...\n");
    printf("  (Try pressing Ctrl+C — signal will be pending)\n");

    sigprocmask(SIG_BLOCK, &mask, &oldmask);
    sleep(2);

    printf("Unblocking SIGINT — pending signal is delivered now.\n");
    sigprocmask(SIG_SETMASK, &oldmask, NULL);

    printf("(Resumed after unblock)\n\n");
}

/* ---------- pitfall_demo ---------- */
static sigjmp_buf jump_env;

static void pitfall_handler(int sig) {
    (void)sig;
    /* This handler uses printf — which is WRONG in production code.
     * Demonstrated here to show what NOT to do. */
    printf("  [handler: unsafe printf — do NOT do this in production]\n");
    siglongjmp(jump_env, 1);
}

static void pitfall_demo(void) {
    printf("=== Pitfall Demo (What NOT To Do) ===\n\n");

    struct sigaction sa;
    sa.sa_handler = pitfall_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = 0;
    sigaction(SIGUSR1, &sa, NULL);

    printf("Sending SIGUSR1 to self...\n");
    printf("  In the handler, we call printf() — which is NOT async-signal-safe.\n");
    printf("  This can corrupt stdio state if it interrupts another printf.\n\n");

    if (sigsetjmp(jump_env, 1) == 0) {
        kill(getpid(), SIGUSR1);
        printf("  (This line is never reached)\n");
    } else {
        printf("  Back from handler via siglongjmp.\n");
        printf("  Lesson: use write() in handlers, not printf().\n\n");
    }

    signal(SIGUSR1, SIG_DFL);
}

int main(void) {
    printf("Signals: Delivery, Handling, Pitfalls\n");
    printf("=======================================\n\n");

    sigint_demo();
    sigsegv_demo();
    sigchld_demo();
    alarm_demo();
    signal_mask_demo();
    pitfall_demo();

    printf("Done.\n");
    return 0;
}
