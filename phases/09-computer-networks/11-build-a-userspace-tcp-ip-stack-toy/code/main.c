#include <stdint.h>
#include <stdio.h>

struct ipv4_header {
  uint8_t version_ihl;
  uint8_t dscp_ecn;
  uint16_t total_len;
  uint16_t ident;
  uint16_t flags_frag;
  uint8_t ttl;
  uint8_t proto;
  uint16_t checksum;
  uint32_t src;
  uint32_t dst;
};

int main(void) {
  struct ipv4_header h = {
      .version_ihl = 0x45,
      .ttl = 64,
      .proto = 6,
      .total_len = 40,
  };
  printf("toy ipv4 packet: vihl=0x%02x ttl=%u proto=%u len=%u\n", h.version_ihl, h.ttl, h.proto, h.total_len);
  return 0;
}
