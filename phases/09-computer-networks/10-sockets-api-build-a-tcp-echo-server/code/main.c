#include <arpa/inet.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

int main(void) {
  int server_fd = socket(AF_INET, SOCK_STREAM, 0);
  if (server_fd < 0) return 1;

  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  addr.sin_port = htons(9090);

  int opt = 1;
  setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
  if (bind(server_fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) return 2;
  if (listen(server_fd, 8) < 0) return 3;

  printf("echo server listening on 127.0.0.1:9090\n");
  close(server_fd);
  return 0;
}
