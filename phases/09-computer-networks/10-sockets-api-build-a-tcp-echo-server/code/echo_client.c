/*
 * echo_client.c — TCP echo client
 * Phase 09 — Computer Networks, Lesson 10
 *
 * Connects to an echo server, reads lines from stdin,
 * sends them, and prints the echoed response.
 *
 * Build:  gcc -o echo_client echo_client.c
 * Run:    ./echo_client [host]
 * Test:   ./echo_client 127.0.0.1
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <arpa/inet.h>

#define PORT 8080
#define BUF_SIZE 4096

static ssize_t read_all(int fd, char *buf, size_t len) {
    size_t total = 0;
    while (total < len) {
        ssize_t n = read(fd, buf + total, len - total);
        if (n < 0) {
            if (errno == EINTR) continue;
            return -1;
        }
        if (n == 0) return total;
        total += n;
    }
    return total;
}

static ssize_t write_all(int fd, const char *buf, size_t len) {
    size_t total = 0;
    while (total < len) {
        ssize_t n = write(fd, buf + total, len - total);
        if (n < 0) {
            if (errno == EINTR) continue;
            return -1;
        }
        total += n;
    }
    return total;
}

int main(int argc, char *argv[]) {
    const char *host = "127.0.0.1";
    if (argc > 1) host = argv[1];

    int sockfd = socket(AF_INET, SOCK_STREAM, 0);
    if (sockfd < 0) {
        perror("socket");
        exit(1);
    }

    struct sockaddr_in server_addr = {
        .sin_family = AF_INET,
        .sin_port = htons(PORT),
    };

    if (inet_pton(AF_INET, host, &server_addr.sin_addr) != 1) {
        fprintf(stderr, "Invalid address: %s\n", host);
        close(sockfd);
        exit(1);
    }

    printf("Connecting to %s:%d...\n", host, PORT);

    if (connect(sockfd, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        perror("connect");
        close(sockfd);
        exit(1);
    }

    printf("Connected. Type messages (Ctrl-D to quit).\n");

    char buf[BUF_SIZE];
    while (1) {
        printf("> ");
        fflush(stdout);

        if (fgets(buf, BUF_SIZE, stdin) == NULL) {
            printf("\n");
            break;
        }

        size_t len = strlen(buf);
        if (len == 0) continue;

        if (write_all(sockfd, buf, len) < 0) {
            perror("write");
            break;
        }

        ssize_t n = read_all(sockfd, buf, BUF_SIZE - 1);
        if (n <= 0) {
            if (n == 0) printf("Server closed connection.\n");
            else perror("read");
            break;
        }
        buf[n] = '\0';
        printf("echo: %s", buf);
    }

    close(sockfd);
    return 0;
}
