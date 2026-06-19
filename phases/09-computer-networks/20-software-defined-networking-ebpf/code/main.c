/**
 * Software-Defined Networking & eBPF
 * Phase 09 — Computer Networks
 *
 * Userspace simulation of an eBPF XDP packet counter.
 * Parses Ethernet/IP/transport headers, counts packets by protocol
 * using simulated BPF_MAP_TYPE_ARRAY, and tracks per-flow byte counts.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* ------------------------------------------------------------------ */
/*  Byte-order helpers (avoid <arpa/inet.h> dependency)               */
/* ------------------------------------------------------------------ */

static inline uint16_t ntoh16(uint16_t x) {
    return ((x & 0xFF) << 8) | ((x >> 8) & 0xFF);
}

static inline uint32_t ntoh32(uint32_t x) {
    return ((x & 0xFF) << 24) | ((x & 0xFF00) << 8) |
           ((x >> 8) & 0xFF00) | ((x >> 24) & 0xFF);
}

/* ------------------------------------------------------------------ */
/*  Simulated packet header structures (packed to match wire format)  */
/* ------------------------------------------------------------------ */

struct eth_hdr {
    uint8_t  dst_mac[6];
    uint8_t  src_mac[6];
    uint16_t eth_type;
} __attribute__((packed));

struct ipv4_hdr {
    uint8_t  version_ihl;
    uint8_t  dscp_ecn;
    uint16_t total_length;
    uint16_t identification;
    uint16_t flags_fragment;
    uint8_t  ttl;
    uint8_t  protocol;
    uint16_t checksum;
    uint32_t src_ip;
    uint32_t dst_ip;
} __attribute__((packed));

/* ------------------------------------------------------------------ */
/*  Simulated BPF maps                                                */
/* ------------------------------------------------------------------ */

#define MAX_PROTO 4
static const char *proto_names[MAX_PROTO] = {"TCP", "UDP", "ICMP", "OTHER"};
static uint64_t proto_counts[MAX_PROTO] = {0};

#define MAX_FLOWS 256
typedef struct {
    uint32_t src_ip;
    uint32_t dst_ip;
    uint8_t  protocol;
    uint16_t src_port;
    uint16_t dst_port;
    uint64_t bytes;
} flow_t;
static flow_t flows[MAX_FLOWS];
static int    flow_cnt = 0;

/* ------------------------------------------------------------------ */
/*  Simulated XDP actions                                             */
/* ------------------------------------------------------------------ */

#define XDP_PASS 1
#define XDP_DROP 0

/* ------------------------------------------------------------------ */
/*  XDP program — mirrors a real eBPF/XDP program that:              */
/*   – parses Ethernet + IPv4 + TCP/UDP/ICMP headers                 */
/*   – increments per-protocol counters (BPF_MAP_TYPE_ARRAY)         */
/*   – updates per-flow byte counters (BPF_MAP_TYPE_HASH)            */
/* ------------------------------------------------------------------ */

int xdp_program(const uint8_t *data, size_t len) {
    /* Parse Ethernet */
    if (len < sizeof(struct eth_hdr))
        return XDP_DROP;
    struct eth_hdr *eth = (struct eth_hdr *)data;
    if (ntoh16(eth->eth_type) != 0x0800)
        return XDP_PASS;               /* non-IPv4: let through */

    /* Parse IPv4 */
    size_t off = sizeof(struct eth_hdr);
    if (len < off + 20)
        return XDP_DROP;
    struct ipv4_hdr *ip = (struct ipv4_hdr *)(data + off);
    int ihl = (ip->version_ihl & 0x0F) * 4;
    if (ihl < 20 || len < off + (size_t)ihl)
        return XDP_DROP;

    int      proto_idx;
    uint16_t src_port = 0, dst_port = 0;

    switch (ip->protocol) {
    case 6:                     /* TCP */
        proto_idx = 0;
        if (len >= off + (size_t)ihl + 4) {
            src_port = (data[off + ihl]     << 8) | data[off + ihl + 1];
            dst_port = (data[off + ihl + 2] << 8) | data[off + ihl + 3];
        }
        break;
    case 17:                    /* UDP */
        proto_idx = 1;
        if (len >= off + (size_t)ihl + 4) {
            src_port = (data[off + ihl]     << 8) | data[off + ihl + 1];
            dst_port = (data[off + ihl + 2] << 8) | data[off + ihl + 3];
        }
        break;
    case 1:                     /* ICMP */
        proto_idx = 2;
        break;
    default:
        proto_idx = 3;
    }

    /* Update protocol counter (BPF_MAP_TYPE_ARRAY increment) */
    __sync_fetch_and_add(&proto_counts[proto_idx], 1);

    /* Track per-flow bytes for TCP/UDP (BPF_MAP_TYPE_HASH semantics) */
    if (proto_idx < 2) {
        int found = 0;
        for (int i = 0; i < flow_cnt; i++) {
            if (flows[i].src_ip   == ip->src_ip  &&
                flows[i].dst_ip   == ip->dst_ip  &&
                flows[i].protocol == ip->protocol &&
                flows[i].src_port == src_port    &&
                flows[i].dst_port == dst_port) {
                flows[i].bytes += ntoh16(ip->total_length);
                found = 1;
                break;
            }
        }
        if (!found && flow_cnt < MAX_FLOWS) {
            flows[flow_cnt].src_ip   = ip->src_ip;
            flows[flow_cnt].dst_ip   = ip->dst_ip;
            flows[flow_cnt].protocol = ip->protocol;
            flows[flow_cnt].src_port = src_port;
            flows[flow_cnt].dst_port = dst_port;
            flows[flow_cnt].bytes    = ntoh16(ip->total_length);
            flow_cnt++;
        }
    }
    return XDP_PASS;
}

/* ------------------------------------------------------------------ */
/*  Test traffic generation                                           */
/* ------------------------------------------------------------------ */

static void build_tcp_pkt(uint8_t *buf, size_t len,
                          const uint8_t *sip, const uint8_t *dip,
                          uint16_t sp, uint16_t dp)
{
    memset(buf, 0, len);
    buf[12] = 0x08; buf[13] = 0x00;               /* EtherType IPv4 */
    buf[14] = 0x45;                               /* ver=4, IHL=5 */
    buf[16] = 0x00; buf[17] = 40;                 /* total_length=40 */
    buf[23] = 6;                                   /* protocol=TCP */
    memcpy(buf + 26, sip, 4);                      /* src IP */
    memcpy(buf + 30, dip, 4);                      /* dst IP */
    buf[34] = (sp >> 8) & 0xFF; buf[35] = sp & 0xFF;
    buf[36] = (dp >> 8) & 0xFF; buf[37] = dp & 0xFF;
}

static void build_udp_pkt(uint8_t *buf, size_t len,
                          const uint8_t *sip, const uint8_t *dip,
                          uint16_t sp, uint16_t dp)
{
    build_tcp_pkt(buf, len, sip, dip, sp, dp);
    buf[16] = 0x00; buf[17] = 30;                 /* smaller packet */
    buf[23] = 17;                                  /* protocol=UDP  */
}

static void build_icmp_pkt(uint8_t *buf, size_t len,
                           const uint8_t *sip, const uint8_t *dip)
{
    build_tcp_pkt(buf, len, sip, dip, 0, 0);
    buf[16] = 0x00; buf[17] = 28;                 /* total_length=28 */
    buf[23] = 1;                                   /* protocol=ICMP */
}

static void simulate_traffic(void) {
    uint8_t buf[64];
    uint8_t sipA[] = {192, 168, 1, 1};
    uint8_t dipB[] = {10,   0,   0, 1};
    uint8_t dns[]  = {8,    8,   8, 8};

    printf("  Processing packets through XDP program...\n");

    /* 20 TCP packets */
    build_tcp_pkt(buf, sizeof(buf), sipA, dipB, 80, 8080);
    for (int i = 0; i < 20; i++) xdp_program(buf, 54);
    printf("    TCP x20: XDP_PASS\n");

    /* 10 UDP DNS packets */
    build_udp_pkt(buf, sizeof(buf), sipA, dns, 53, 12345);
    for (int i = 0; i < 10; i++) xdp_program(buf, 54);
    printf("    UDP x10: XDP_PASS\n");

    /* 5 ICMP pings */
    build_icmp_pkt(buf, sizeof(buf), sipA, dns);
    for (int i = 0; i < 5; i++) xdp_program(buf, 54);
    printf("    ICMP  x5: XDP_PASS\n");
}

/* ------------------------------------------------------------------ */
/*  main                                                              */
/* ------------------------------------------------------------------ */

int main(void) {
    printf("=== Software-Defined Networking & eBPF ===\n\n");

    printf("SDN simulation: see python3 code/main.py\n");
    printf("- 3-switch tree topology\n");
    printf("- Learning switch (MAC learning via PACKET_IN)\n");
    printf("- Static flow rule installation via controller\n\n");

    printf("=== XDP Packet Counter (in-C eBPF simulation) ===\n\n");
    simulate_traffic();

    printf("\n  Protocol distribution (BPF_MAP_TYPE_ARRAY):\n");
    for (int i = 0; i < MAX_PROTO; i++) {
        int bar = proto_counts[i] > 20 ? 20 : (int)proto_counts[i];
        printf("    %-6s: %3llu pkts ", proto_names[i],
               (unsigned long long)proto_counts[i]);
        for (int j = 0; j < bar; j++) putchar('#');
        putchar('\n');
    }

    printf("\n  Top flows by byte volume:\n");
    int shown = 0;
    for (int i = 0; i < flow_cnt && shown < 5; i++) {
        uint32_t sip = ntoh32(flows[i].src_ip);
        uint32_t dip = ntoh32(flows[i].dst_ip);
        const char *pname = proto_names[
            flows[i].protocol == 6  ? 0 :
            flows[i].protocol == 17 ? 1 :
            flows[i].protocol == 1  ? 2 : 3];
        printf("    %d.%d.%d.%d:%-5d -> %d.%d.%d.%d:%-5d (%s): %llu bytes\n",
               (sip >> 24) & 0xFF, (sip >> 16) & 0xFF,
               (sip >> 8)  & 0xFF, sip & 0xFF, flows[i].src_port,
               (dip >> 24) & 0xFF, (dip >> 16) & 0xFF,
               (dip >> 8)  & 0xFF, dip & 0xFF, flows[i].dst_port,
               pname, (unsigned long long)flows[i].bytes);
        shown++;
    }
    putchar('\n');
    return 0;
}
