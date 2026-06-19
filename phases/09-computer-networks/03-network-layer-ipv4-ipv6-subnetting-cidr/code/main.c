/*
 * IPv4 Header Parser
 *
 * Parses raw IPv4 packet bytes, prints all header fields,
 * verifies the header checksum, and identifies the protocol.
 *
 * Compile: gcc -o ip_parser main.c
 * Run:     ./ip_parser
 */

#include <stdio.h>
#include <stdint.h>
#include <string.h>

typedef struct {
    uint8_t  version;
    uint8_t  ihl;              /* Header length in 32-bit words */
    uint8_t  dscp_ecn;
    uint16_t total_length;
    uint16_t identification;
    uint16_t flags_fragment;
    uint8_t  ttl;
    uint8_t  protocol;
    uint16_t checksum;
    uint32_t src_ip;
    uint32_t dst_ip;
    uint8_t  options[40];      /* Up to 40 bytes of options */
    size_t   options_len;
    uint8_t  payload[65535];
    size_t   payload_len;
} IPv4Header;

static const char *protocol_name(uint8_t proto) {
    switch (proto) {
        case 1:  return "ICMP";
        case 6:  return "TCP";
        case 17: return "UDP";
        case 47: return "GRE";
        case 50: return "ESP";
        case 89: return "OSPF";
        default: return "Unknown";
    }
}

static void print_ip(uint32_t ip) {
    printf("%d.%d.%d.%d",
           (ip >> 24) & 0xFF,
           (ip >> 16) & 0xFF,
           (ip >> 8) & 0xFF,
           ip & 0xFF);
}

static uint16_t ip_checksum(const uint8_t *data, size_t len) {
    uint32_t sum = 0;
    /* Sum 16-bit words */
    for (size_t i = 0; i + 1 < len; i += 2) {
        sum += ((uint16_t)data[i] << 8) | data[i + 1];
    }
    /* Handle odd byte */
    if (len & 1) {
        sum += (uint16_t)data[len - 1] << 8;
    }
    /* Fold 32-bit sum to 16 bits */
    while (sum >> 16) {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    return (uint16_t)(~sum);
}

int parse_ipv4_packet(const uint8_t *raw, size_t raw_len) {
    IPv4Header hdr;

    if (raw_len < 20) {
        fprintf(stderr, "Error: packet too short (%zu bytes)\n", raw_len);
        return -1;
    }

    hdr.version = (raw[0] >> 4) & 0x0F;
    hdr.ihl = raw[0] & 0x0F;
    hdr.dscp_ecn = raw[1];
    hdr.total_length = ((uint16_t)raw[2] << 8) | raw[3];
    hdr.identification = ((uint16_t)raw[4] << 8) | raw[5];
    hdr.flags_fragment = ((uint16_t)raw[6] << 8) | raw[7];
    hdr.ttl = raw[8];
    hdr.protocol = raw[9];
    hdr.checksum = ((uint16_t)raw[10] << 8) | raw[11];
    hdr.src_ip = ((uint32_t)raw[12] << 24) | ((uint32_t)raw[13] << 16)
               | ((uint32_t)raw[14] << 8) | raw[15];
    hdr.dst_ip = ((uint32_t)raw[16] << 24) | ((uint32_t)raw[17] << 16)
               | ((uint32_t)raw[18] << 8) | raw[19];

    size_t header_bytes = hdr.ihl * 4;
    if (header_bytes < 20 || header_bytes > 60) {
        fprintf(stderr, "Error: invalid IHL (%u words = %zu bytes)\n", hdr.ihl, header_bytes);
        return -1;
    }
    if (header_bytes > raw_len) {
        fprintf(stderr, "Error: header length exceeds packet\n");
        return -1;
    }

    hdr.options_len = header_bytes - 20;
    if (hdr.options_len > 0) {
        memcpy(hdr.options, raw + 20, hdr.options_len);
    }

    if (hdr.total_length > raw_len) {
        fprintf(stderr, "Warning: total_length (%u) > buffer (%zu)\n",
                hdr.total_length, raw_len);
    }

    hdr.payload_len = hdr.total_length > header_bytes ? hdr.total_length - header_bytes : 0;
    if (hdr.payload_len > 0 && header_bytes + hdr.payload_len <= raw_len) {
        memcpy(hdr.payload, raw + header_bytes, hdr.payload_len);
    }

    /* Verify checksum: compute over the header as-is (should yield 0 if valid) */
    uint8_t hdr_copy[60];
    memcpy(hdr_copy, raw, header_bytes);
    uint16_t computed = ip_checksum(hdr_copy, header_bytes);

    /* Print fields */
    printf("=== IPv4 Packet ===\n");
    printf("Version         : %u\n", hdr.version);
    printf("Header Length   : %u words (%zu bytes)\n", hdr.ihl, header_bytes);
    printf("DSCP/ECN        : 0x%02x\n", hdr.dscp_ecn);
    printf("Total Length    : %u bytes\n", hdr.total_length);
    printf("Identification  : %u (0x%04x)\n", hdr.identification, hdr.identification);

    uint16_t flags = (hdr.flags_fragment >> 13) & 0x07;
    uint16_t frag_offset = hdr.flags_fragment & 0x1FFF;
    printf("Flags           : 0x%01x (DF=%d, MF=%d)\n",
           flags, (flags >> 1) & 1, flags & 1);
    printf("Fragment Offset : %u\n", frag_offset);
    printf("TTL             : %u\n", hdr.ttl);
    printf("Protocol        : %u (%s)\n", hdr.protocol, protocol_name(hdr.protocol));
    printf("Header Checksum : 0x%04x\n", hdr.checksum);
    printf("Computed Checksum: 0x%04x\n", computed);
    printf("Checksum valid  : %s\n", computed == 0 ? "PASS" : "FAIL");
    printf("Source IP       : "); print_ip(hdr.src_ip); printf("\n");
    printf("Destination IP  : "); print_ip(hdr.dst_ip); printf("\n");

    if (hdr.options_len > 0) {
        printf("Options         : (%zu bytes) ", hdr.options_len);
        for (size_t i = 0; i < hdr.options_len && i < 16; i++) {
            printf("%02x ", hdr.options[i]);
        }
        if (hdr.options_len > 16) printf("...");
        printf("\n");
    }

    printf("Payload length  : %zu bytes\n", hdr.payload_len);
    if (hdr.payload_len > 0) {
        printf("Payload (hex)   : ");
        size_t show = hdr.payload_len < 32 ? hdr.payload_len : 32;
        for (size_t i = 0; i < show; i++) {
            printf("%02x ", hdr.payload[i]);
        }
        if (hdr.payload_len > 32) printf("...");
        printf("\n");
    }

    return 0;
}

int main(void) {
    /* Sample IPv4 packet: TCP SYN to 8.8.8.8 from 192.168.1.50 */
    uint8_t packet[] = {
        /* Version=4, IHL=5 (20 bytes), DSCP=0 */
        0x45, 0x00,
        /* Total Length = 40 (20 header + 20 TCP) */
        0x00, 0x28,
        /* Identification */
        0x12, 0x34,
        /* Flags=0x4000 (DF=1), Fragment Offset=0 */
        0x40, 0x00,
        /* TTL=64 */
        0x40,
        /* Protocol=6 (TCP) */
        0x06,
        /* Header Checksum (placeholder, computed below) */
        0x00, 0x00,
        /* Source IP: 192.168.1.50 */
        0xC0, 0xA8, 0x01, 0x32,
        /* Destination IP: 8.8.8.8 */
        0x08, 0x08, 0x08, 0x08,
        /* TCP header (simplified, 20 bytes) */
        0x04, 0xD2,  /* Src port: 1234 */
        0x01, 0xBB,  /* Dst port: 443 */
        0x00, 0x00, 0x00, 0x01, /* Seq number */
        0x00, 0x00, 0x00, 0x00, /* Ack number */
        0x50, 0x02,  /* Data offset=5, flags=SYN */
        0xFF, 0xFF,  /* Window */
        0x00, 0x00,  /* Checksum */
        0x00, 0x00,  /* Urgent pointer */
    };

    /* Compute and fill in the IP header checksum */
    uint8_t hdr_copy[20];
    memcpy(hdr_copy, packet, 20);
    hdr_copy[10] = 0;
    hdr_copy[11] = 0;
    uint16_t cksum = ip_checksum(hdr_copy, 20);
    packet[10] = (cksum >> 8) & 0xFF;
    packet[11] = cksum & 0xFF;

    parse_ipv4_packet(packet, sizeof(packet));

    return 0;
}
