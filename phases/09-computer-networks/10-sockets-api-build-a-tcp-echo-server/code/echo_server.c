/*
 * echo_server.c — Multi-client TCP echo server
 * Phase 09 — Computer Networks, Lesson 10
 *
 * Listens on port 8080, accepts connections, echoes data back.
 * Handles multiple clients via fork() (one process per client).
 * Clean shutdown on SIGINT.
 *
 * Build:  gcc -o echo_server echo_server.c
 * Run:    ./echo_server
 * Test:   echo "hello" | nc localhost 8080
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <signal.h>
#include <errno.h>
#include <sys/wait.h>
#include <arpa/inet.h>

#define PORT 8080
#define BUF_SIZE 4096
#define BACKLOG 128

static volatile sig_atomic_t running = 1;
static int server_fd = -1;

static void sigchld_handler(int sig) {
    (void)sig;
    /* Reap zombie child processes */
    while (waitpid(-1, NULL, WNOHANG) > 0);
}

static void sigint_handler(int sig) {
    (void)sig;
    running = 0;
    if (server_fd >= 0) close(server_fd);
    fprintf(stderr, "\nShutting down.\n");
    _exit(0);
}

static void handle_client(int client_fd) {
    char buf[BUF_SIZE];
    ssize_t n;

    while ((n = read(client_fd, buf, BUF_SIZE)) > 0) {
        ssize_t total_written = 0;
        while (total_written < n) {
            ssize_t written = write(client_fd, buf + total_written, n - total_written);
            if (written < 0) {
                if (errno == EINTR) continue;
                perror("write");
                break;
            }
            total_written += written;
        }
    }

    if (n < 0) perror("read");
    close(client_fd);
}

int main(void) {
    struct sigaction sa_chld = { .sa_handler = sigchld_handler };
    sigemptyset(&sa_chld.sa_mask);
    sa_chld.sa_flags = SA_RESTART;
    sigaction(SIGCHLD, &sa_chld, NULL);

    struct sigaction sa_int = { .sa_handler = sigint_handler };
    sigemptyset(&sa_int.sa_mask);
    sigaction(SIGINT, &sa_int, NULL);

    server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd < 0) {
        perror("socket");
        exit(1);
    }

    int opt = 1;
    if (setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0) {
        perror("setsockopt");
        close(server_fd);
        exit(1);
    }

    struct sockaddr_in addr = {
        .sin_family = AF_INET,
        .sin_port = htons(PORT),
        .sin_addr.s_addr = INADDR_ANY
    };

    if (bind(server_fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("bind");
        close(server_fd);
        exit(1);
    }

    if (listen(server_fd, BACKLOG) < 0) {
        perror("listen");
        close(server_fd);
        exit(1);
    }

    printf("Echo server listening on 0.0.0.0:%d (PID %d)\n", PORT, getpid());

    while (running) {
        struct sockaddr_in client_addr;
        socklen_t client_len = sizeof(client_addr);

        int client_fd = accept(server_fd, (struct sockaddr *)&client_addr, &client_len);
        if (client_fd < 0) {
            if (errno == EINTR) continue;
            perror("accept");
            continue;
        }

        char ip_str[INET_ADDRSTRLEN];
        inet_ntop(AF_INET, &client_addr.sin_addr, ip_str, sizeof(ip_str));
        printf("Connection from %s:%d\n", ip_str, ntohs(client_addr.sin_port));

        pid_t pid = fork();
        if (pid < 0) {
            perror("fork");
            close(client_fd);
            continue;
        }

        if (pid == 0) {
            /* Child process */
            close(server_fd);
            handle_client(client_fd);
            exit(0);
        }

        /* Parent process */
        close(client_fd);
    }

    close(server_fd);
    return 0;
}
