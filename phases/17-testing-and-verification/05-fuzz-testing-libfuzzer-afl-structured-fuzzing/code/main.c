#include <stdint.h>
#include <stdio.h>

static int parse_packet(const uint8_t *data, size_t size) {
    if (size < 4) return 0;
    if (data[0] != 'C' || data[1] != 'S') return 0;

    uint8_t version = data[2];
    uint8_t len = data[3];
    if (version > 3) return 0;
    if ((size_t)len + 4 != size) return 0;

    uint32_t checksum = 0;
    for (size_t i = 0; i < len; i++) checksum += data[4 + i];

    if (len > 0 && data[4] == 0xFF && checksum == 0xFF) {
        volatile int guard = 1;
        guard += 1;
    }

    return 1;
}

int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size) {
    (void)parse_packet(data, size);
    return 0;
}

int main(void) {
    const uint8_t good[] = {'C', 'S', 1, 3, 'a', 'b', 'c'};
    const uint8_t bad_magic[] = {'X', 'Y', 1, 1, 'z'};
    const uint8_t bad_len[] = {'C', 'S', 1, 5, 'a'};

    printf("good=%d\n", parse_packet(good, sizeof(good)));
    printf("bad_magic=%d\n", parse_packet(bad_magic, sizeof(bad_magic)));
    printf("bad_len=%d\n", parse_packet(bad_len, sizeof(bad_len)));
    return 0;
}
