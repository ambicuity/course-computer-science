/*
 * Transport — UDP, TCP State Machine
 * Phase 09 — Computer Networks
 *
 * UDP echo server/client and TCP connection simulation.
 * Compile: gcc -Wall -Wextra -o transport main.c
 * Run server: ./transport server 9000
 * Run client: ./transport client 127.0.0.1 9000 "hello"
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <arpa/inet.h>
#include <sys/socket.h>

#define BUF_SIZE 4096
#define DEFAULT_PORT 9000

/* ── Packet printing (Wireshark-like) ────────────────────────────── */

static void print_hex(const unsigned char *data, size_t len, size_t offset) {
    for (size_t i = 0; i < len; i++) {
        if (i % 16 == 0) printf("  %04zx: ", offset + i);
        printf("%02x ", data[i]);
        if (i % 16 == 15 || i == len - 1) {
            size_t pad = 15 - (i % 16);
            for (size_t p = 0; p < pad; p++) printf("   ");
            printf(" |");
            for (size_t j = i - (i % 16); j <= i; j++) {
                printf("%c", (data[j] >= 32 && data[j] < 127) ? data[j] : '.');
            }
            printf("|\n");
        }
    }
}

static void print_packet(const char *direction, const char *proto,
                         const char *src, const char *dst,
                         const unsigned char *data, size_t len) {
    printf("[%s] %s %s → %s (%zu bytes)\n", direction, proto, src, dst, len);
    print_hex(data, len, 0);
    printf("\n");
}

/* ── UDP Echo Server ─────────────────────────────────────────────── */

static void udp_server(int port) {
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd < 0) { perror("socket"); exit(1); }

    int opt = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in addr = {
        .sin_family = AF_INET,
        .sin_port = htons(port),
        .sin_addr.s_addr = htonl(INADDR_ANY)
    };

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("bind");
        close(fd);
        exit(1);
    }

    printf("UDP echo server listening on port %d\n", port);

    unsigned char buf[BUF_SIZE];
    struct sockaddr_in client;
    socklen_t client_len = sizeof(client);

    while (1) {
        ssize_t n = recvfrom(fd, buf, BUF_SIZE, 0,
                             (struct sockaddr *)&client, &client_len);
        if (n < 0) { perror("recvfrom"); continue; }

        char client_ip[INET_ADDRSTRLEN];
        inet_ntop(AF_INET, &client.sin_addr, client_ip, sizeof(client_ip));

        print_packet("RECV", "UDP", client_ip, "server", buf, (size_t)n);

        ssize_t sent = sendto(fd, buf, (size_t)n, 0,
                              (struct sockaddr *)&client, client_len);
        if (sent < 0) perror("sendto");

        print_packet("SEND", "UDP", "server", client_ip, buf, (size_t)n);
    }

    close(fd);
}

/* ── UDP Echo Client ─────────────────────────────────────────────── */

static void udp_client(const char *host, int port, const char *message) {
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd < 0) { perror("socket"); exit(1); }

    struct sockaddr_in server = {
        .sin_family = AF_INET,
        .sin_port = htons(port),
    };

    if (inet_pton(AF_INET, host, &server.sin_addr) <= 0) {
        fprintf(stderr, "Invalid address: %s\n", host);
        close(fd);
        exit(1);
    }

    size_t len = strlen(message);

    print_packet("SEND", "UDP", "client", host, (const unsigned char *)message, len);

    ssize_t sent = sendto(fd, message, len, 0,
                          (struct sockaddr *)&server, sizeof(server));
    if (sent < 0) { perror("sendto"); close(fd); exit(1); }

    unsigned char buf[BUF_SIZE];
    struct sockaddr_in from;
    socklen_t from_len = sizeof(from);

    ssize_t n = recvfrom(fd, buf, BUF_SIZE, 0,
                         (struct sockaddr *)&from, &from_len);
    if (n < 0) { perror("recvfrom"); close(fd); exit(1); }

    print_packet("RECV", "UDP", host, "client", buf, (size_t)n);
    printf("Echo: %.*s\n", (int)n, buf);

    close(fd);
}

/* ── TCP Server ──────────────────────────────────────────────────── */

static void tcp_server(int port) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) { perror("socket"); exit(1); }

    int opt = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in addr = {
        .sin_family = AF_INET,
        .sin_port = htons(port),
        .sin_addr.s_addr = htonl(INADDR_ANY)
    };

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("bind"); close(fd); exit(1);
    }
    if (listen(fd, 5) < 0) {
        perror("listen"); close(fd); exit(1);
    }

    printf("TCP server listening on port %d\n", port);

    while (1) {
        struct sockaddr_in client;
        socklen_t client_len = sizeof(client);
        int cfd = accept(fd, (struct sockaddr *)&client, &client_len);
        if (cfd < 0) { perror("accept"); continue; }

        char client_ip[INET_ADDRSTRLEN];
        inet_ntop(AF_INET, &client.sin_addr, client_ip, sizeof(client_ip));
        printf("[TCP] Connection from %s:%d — ESTABLISHED\n", client_ip, ntohs(client.sin_port));

        unsigned char buf[BUF_SIZE];
        ssize_t n;
        while ((n = read(cfd, buf, BUF_SIZE)) > 0) {
            print_packet("RECV", "TCP", client_ip, "server", buf, (size_t)n);
            write(cfd, buf, (size_t)n);
            print_packet("SEND", "TCP", "server", client_ip, buf, (size_t)n);
        }

        printf("[TCP] Connection from %s closed\n", client_ip);
        close(cfd);
    }

    close(fd);
}

/* ── TCP Client ──────────────────────────────────────────────────── */

static void tcp_client(const char *host, int port, const char *message) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) { perror("socket"); exit(1); }

    struct sockaddr_in server = {
        .sin_family = AF_INET,
        .sin_port = htons(port),
    };

    if (inet_pton(AF_INET, host, &server.sin_addr) <= 0) {
        fprintf(stderr, "Invalid address: %s\n", host);
        close(fd);
        exit(1);
    }

    printf("[TCP] Connecting to %s:%d ...\n", host, port);
    if (connect(fd, (struct sockaddr *)&server, sizeof(server)) < 0) {
        perror("connect"); close(fd); exit(1);
    }
    printf("[TCP] Connected — ESTABLISHED (three-way handshake complete)\n");

    size_t len = strlen(message);
    print_packet("SEND", "TCP", "client", host, (const unsigned char *)message, len);
    write(fd, message, len);

    unsigned char buf[BUF_SIZE];
    ssize_t n = read(fd, buf, BUF_SIZE);
    if (n > 0) {
        print_packet("RECV", "TCP", host, "client", buf, (size_t)n);
        printf("Echo: %.*s\n", (int)n, buf);
    }

    printf("[TCP] Sending FIN — entering FIN_WAIT_1\n");
    close(fd);
    printf("[TCP] Connection closed\n");
}

/* ── Main ────────────────────────────────────────────────────────── */

int main(int argc, char *argv[]) {
    if (argc < 3) {
        fprintf(stderr, "Usage:\n");
        fprintf(stderr, "  %s server <port>\n", argv[0]);
        fprintf(stderr, "  %s client <host> <port> <message>\n", argv[0]);
        return 1;
    }

    if (strcmp(argv[1], "server") == 0) {
        int port = atoi(argv[2]);
        printf("Choose protocol: 1=UDP, 2=TCP [default=UDP]: ");
        int proto = 0;
        if (scanf("%d", &proto) != 1) proto = 1;
        if (proto == 2) tcp_server(port);
        else udp_server(port);
    } else if (strcmp(argv[1], "client") == 0) {
        if (argc < 5) {
            fprintf(stderr, "Usage: %s client <host> <port> <message>\n", argv[0]);
            return 1;
        }
        const char *host = argv[2];
        int port = atoi(argv[3]);
        const char *msg = argv[4];
        printf("Choose protocol: 1=UDP, 2=TCP [default=UDP]: ");
        int proto = 0;
        if (scanf("%d", &proto) != 1) proto = 1;
        if (proto == 2) tcp_client(host, port, msg);
        else udp_client(host, port, msg);
    } else {
        fprintf(stderr, "Unknown mode: %s\n", argv[1]);
        return 1;
    }

    return 0;
}
