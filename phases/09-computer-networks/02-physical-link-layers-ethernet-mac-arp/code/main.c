/*
 * Ethernet Frame Parser
 *
 * Parses a raw Ethernet frame and prints its fields.
 * Also performs CRC-32 verification on the frame check sequence.
 *
 * Compile: gcc -o eth_parser main.c
 * Run:     ./eth_parser
 */

#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <arpa/inet.h>

#define ETH_HEADER_LEN  14
#define ETH_FCS_LEN     4
#define ETH_MIN_PAYLOAD 46
#define ETH_MTU         1500
#define ETH_MIN_FRAME   64
#define ETH_MAX_FRAME   1518

/* EtherType values */
#define ETHERTYPE_IPV4 0x0800
#define ETHERTYPE_ARP  0x0806
#define ETHERTYPE_IPV6 0x86DD

typedef struct {
    uint8_t  dst_mac[6];
    uint8_t  src_mac[6];
    uint16_t ether_type;
    uint8_t  payload[ETH_MTU];
    size_t   payload_len;
    uint32_t fcs;
} EthernetFrame;

/* CRC-32 (IEEE 802.3) polynomial: 0xEDB88320 */
static uint32_t crc32_table[256];
static int crc32_table_ready = 0;

static void crc32_init(void) {
    for (uint32_t i = 0; i < 256; i++) {
        uint32_t crc = i;
        for (int j = 0; j < 8; j++) {
            crc = (crc >> 1) ^ (crc & 1 ? 0xEDB88320 : 0);
        }
        crc32_table[i] = crc;
    }
    crc32_table_ready = 1;
}

static uint32_t crc32_compute(const uint8_t *data, size_t len) {
    if (!crc32_table_ready) crc32_init();
    uint32_t crc = 0xFFFFFFFF;
    for (size_t i = 0; i < len; i++) {
        crc = crc32_table[(crc ^ data[i]) & 0xFF] ^ (crc >> 8);
    }
    return crc ^ 0xFFFFFFFF;
}

static void print_mac(const uint8_t mac[6]) {
    printf("%02x:%02x:%02x:%02x:%02x:%02x",
           mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
}

static const char *ether_type_name(uint16_t type) {
    switch (type) {
        case ETHERTYPE_IPV4: return "IPv4 (0x0800)";
        case ETHERTYPE_ARP:  return "ARP  (0x0806)";
        case ETHERTYPE_IPV6: return "IPv6 (0x86DD)";
        default:             return "Unknown";
    }
}

int parse_ethernet_frame(const uint8_t *raw, size_t raw_len) {
    EthernetFrame frame;

    if (raw_len < ETH_HEADER_LEN + ETH_FCS_LEN) {
        fprintf(stderr, "Error: frame too short (%zu bytes)\n", raw_len);
        return -1;
    }

    /* Parse header */
    memcpy(frame.dst_mac, raw, 6);
    memcpy(frame.src_mac, raw + 6, 6);
    frame.ether_type = (uint16_t)(raw[12] << 8) | raw[13];

    /* Parse payload */
    frame.payload_len = raw_len - ETH_HEADER_LEN - ETH_FCS_LEN;
    if (frame.payload_len > ETH_MTU) {
        fprintf(stderr, "Error: payload exceeds MTU (%zu bytes)\n", frame.payload_len);
        return -1;
    }
    memcpy(frame.payload, raw + ETH_HEADER_LEN, frame.payload_len);

    /* Parse FCS (last 4 bytes, little-endian) */
    frame.fcs = (uint32_t)raw[raw_len - 4]
              | ((uint32_t)raw[raw_len - 3] << 8)
              | ((uint32_t)raw[raw_len - 2] << 16)
              | ((uint32_t)raw[raw_len - 1] << 24);

    /* Compute CRC-32 over dst MAC through payload (everything before FCS) */
    uint32_t computed_crc = crc32_compute(raw, raw_len - ETH_FCS_LEN);

    /* Print fields */
    printf("=== Ethernet Frame ===\n");
    printf("Destination MAC : "); print_mac(frame.dst_mac); printf("\n");
    printf("Source MAC      : "); print_mac(frame.src_mac); printf("\n");
    printf("EtherType       : %s\n", ether_type_name(frame.ether_type));
    printf("Payload length  : %zu bytes\n", frame.payload_len);
    printf("FCS (stored)    : 0x%08x\n", frame.fcs);
    printf("FCS (computed)  : 0x%08x\n", computed_crc);
    printf("CRC check       : %s\n",
           frame.fcs == computed_crc ? "PASS" : "FAIL");

    /* Print first 32 bytes of payload as hex */
    printf("Payload (hex)   : ");
    size_t show = frame.payload_len < 32 ? frame.payload_len : 32;
    for (size_t i = 0; i < show; i++) {
        printf("%02x ", frame.payload[i]);
    }
    if (frame.payload_len > 32) printf("...");
    printf("\n");

    return 0;
}

int main(void) {
    /* Sample Ethernet frame (ARP request, broadcast)
     * Dst: ff:ff:ff:ff:ff:ff
     * Src: de:ad:be:ef:00:01
     * EtherType: 0x0806 (ARP)
     * Payload: minimal ARP (28 bytes) + padding (18 bytes)
     * FCS: computed at runtime
     */
    uint8_t frame_data[] = {
        /* Destination MAC (broadcast) */
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        /* Source MAC */
        0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01,
        /* EtherType: ARP */
        0x08, 0x06,
        /* ARP payload (simplified, 28 bytes) */
        0x00, 0x01,                         /* Hardware type: Ethernet */
        0x08, 0x00,                         /* Protocol type: IPv4 */
        0x06,                               /* Hardware size: 6 */
        0x04,                               /* Protocol size: 4 */
        0x00, 0x01,                         /* Opcode: request */
        0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, /* Sender MAC */
        0xC0, 0xA8, 0x01, 0x0A,            /* Sender IP: 192.168.1.10 */
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* Target MAC: unknown */
        0xC0, 0xA8, 0x01, 0x14,            /* Target IP: 192.168.1.20 */
        /* Padding to reach minimum 46-byte payload */
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    };

    /* Compute and append FCS */
    size_t frame_len = sizeof(frame_data);
    uint32_t fcs = crc32_compute(frame_data, frame_len);

    /* Build complete frame with FCS appended */
    uint8_t full_frame[sizeof(frame_data) + 4];
    memcpy(full_frame, frame_data, frame_len);
    full_frame[frame_len]     = (uint8_t)(fcs & 0xFF);
    full_frame[frame_len + 1] = (uint8_t)((fcs >> 8) & 0xFF);
    full_frame[frame_len + 2] = (uint8_t)((fcs >> 16) & 0xFF);
    full_frame[frame_len + 3] = (uint8_t)((fcs >> 24) & 0xFF);

    parse_ethernet_frame(full_frame, sizeof(full_frame));

    return 0;
}
