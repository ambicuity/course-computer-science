#include <stdint.h>
#include <stdio.h>

struct dns_header {
  uint16_t id;
  uint16_t flags;
  uint16_t qdcount;
  uint16_t ancount;
  uint16_t nscount;
  uint16_t arcount;
};

int main(void) {
  struct dns_header q = {.id = 0x1234, .flags = 0x0100, .qdcount = 1};
  printf("dns query header id=0x%04x rd=%u qd=%u\n", q.id, q.flags & 0x0100 ? 1 : 0, q.qdcount);
  return 0;
}
