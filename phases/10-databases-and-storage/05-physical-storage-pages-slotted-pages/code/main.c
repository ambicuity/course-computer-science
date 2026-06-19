/*
 * Physical Storage -- Pages, Slotted Pages
 * Phase 10 -- Databases & Storage Systems
 *
 * Compile: gcc -Wall -Wextra -o main main.c
 * Run:     ./main
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>

#define PAGE_SIZE 4096
#define HEADER_SIZE 24
#define SLOT_ENTRY_SIZE 4

/* ---- helpers: unaligned little-endian I/O ---- */

static uint16_t read_u16(const uint8_t *buf, int off) {
    return (uint16_t)buf[off] | ((uint16_t)buf[off + 1] << 8);
}

static void write_u16(uint8_t *buf, int off, uint16_t v) {
    buf[off]     = v & 0xFF;
    buf[off + 1] = (v >> 8) & 0xFF;
}

static uint32_t read_u32(const uint8_t *buf, int off) {
    return (uint32_t)buf[off]       | ((uint32_t)buf[off + 1] << 8) |
           ((uint32_t)buf[off + 2] << 16) | ((uint32_t)buf[off + 3] << 24);
}

static void write_u32(uint8_t *buf, int off, uint32_t v) {
    buf[off]     = v & 0xFF;
    buf[off + 1] = (v >> 8) & 0xFF;
    buf[off + 2] = (v >> 16) & 0xFF;
    buf[off + 3] = (v >> 24) & 0xFF;
}

/* ---- header offsets ---- */
enum { OFF_PAGE_ID = 0, OFF_FREE_START = 4, OFF_DATA_END = 6, OFF_SLOT_COUNT = 8 };

/* ---- SlottedPage ---- */

typedef struct {
    uint8_t buffer[PAGE_SIZE];
} SlottedPage;

void slotted_page_init(SlottedPage *page, uint32_t page_id) {
    memset(page->buffer, 0, PAGE_SIZE);
    write_u32(page->buffer, OFF_PAGE_ID, page_id);
    write_u16(page->buffer, OFF_FREE_START, HEADER_SIZE);
    write_u16(page->buffer, OFF_DATA_END, PAGE_SIZE);
    write_u16(page->buffer, OFF_SLOT_COUNT, 0);
}

uint32_t slotted_page_id(const SlottedPage *page) {
    return read_u32(page->buffer, OFF_PAGE_ID);
}

static uint16_t free_start(const SlottedPage *page) {
    return read_u16(page->buffer, OFF_FREE_START);
}
static void set_free_start(SlottedPage *page, uint16_t v) {
    write_u16(page->buffer, OFF_FREE_START, v);
}

static uint16_t data_end(const SlottedPage *page) {
    return read_u16(page->buffer, OFF_DATA_END);
}
static void set_data_end(SlottedPage *page, uint16_t v) {
    write_u16(page->buffer, OFF_DATA_END, v);
}

static uint16_t slot_count(const SlottedPage *page) {
    return read_u16(page->buffer, OFF_SLOT_COUNT);
}
static void set_slot_count(SlottedPage *page, uint16_t v) {
    write_u16(page->buffer, OFF_SLOT_COUNT, v);
}

static int free_space(const SlottedPage *page) {
    return (int)data_end(page) - (int)free_start(page);
}

/* ---- slot array accessors ---- */

static int slot_entry_off(int slot) {
    return HEADER_SIZE + slot * SLOT_ENTRY_SIZE;
}

static uint16_t slot_off(const SlottedPage *page, int slot) {
    return read_u16(page->buffer, slot_entry_off(slot));
}
static void set_slot_off(SlottedPage *page, int slot, uint16_t v) {
    write_u16(page->buffer, slot_entry_off(slot), v);
}

static uint16_t slot_len(const SlottedPage *page, int slot) {
    return read_u16(page->buffer, slot_entry_off(slot) + 2);
}
static void set_slot_len(SlottedPage *page, int slot, uint16_t v) {
    write_u16(page->buffer, slot_entry_off(slot) + 2, v);
}

static bool slot_is_deleted(const SlottedPage *page, int slot) {
    return slot_off(page, slot) == 0 && slot_len(page, slot) == 0;
}

/* ---- forward declaration ---- */
void slotted_page_defrag(SlottedPage *page);

/* ---- public API ---- */

int slotted_page_insert(SlottedPage *page, const uint8_t *data, int len) {
    int needed = len + SLOT_ENTRY_SIZE;
    if (needed > free_space(page)) {
        slotted_page_defrag(page);
        if (needed > free_space(page)) return -1;
    }
    uint16_t slot = slot_count(page);
    uint16_t new_de = data_end(page) - (uint16_t)len;
    memcpy(page->buffer + new_de, data, (size_t)len);
    set_slot_off(page, slot, new_de);
    set_slot_len(page, slot, (uint16_t)len);
    set_data_end(page, new_de);
    set_slot_count(page, slot + 1);
    set_free_start(page, (uint16_t)(HEADER_SIZE + slot_count(page) * SLOT_ENTRY_SIZE));
    return (int)slot;
}

const uint8_t *slotted_page_get(const SlottedPage *page, int slot, int *out_len) {
    if (slot < 0 || slot >= (int)slot_count(page)) { *out_len = 0; return NULL; }
    if (slot_is_deleted(page, slot)) { *out_len = 0; return NULL; }
    *out_len = (int)slot_len(page, slot);
    return page->buffer + slot_off(page, slot);
}

void slotted_page_delete(SlottedPage *page, int slot) {
    if (slot < 0 || slot >= (int)slot_count(page)) return;
    set_slot_off(page, slot, 0);
    set_slot_len(page, slot, 0);
}

int slotted_page_update(SlottedPage *page, int slot, const uint8_t *data, int len) {
    if (slot < 0 || slot >= (int)slot_count(page)) return -1;
    if (slot_is_deleted(page, slot)) return -1;

    uint16_t old_off = slot_off(page, slot);
    uint16_t old_len = slot_len(page, slot);

    if (len <= (int)old_len) {
        memcpy(page->buffer + old_off, data, (size_t)len);
        if (len < (int)old_len)
            memset(page->buffer + old_off + len, 0, (size_t)(old_len - len));
        set_slot_len(page, slot, (uint16_t)len);
        return 0;
    }

    /* Grow: delete old slot, relocate to end */
    set_slot_off(page, slot, 0);
    set_slot_len(page, slot, 0);

    int needed = len + SLOT_ENTRY_SIZE;
    if (needed > free_space(page)) {
        slotted_page_defrag(page);
        if (needed > free_space(page)) return -1;
    }
    uint16_t new_de = data_end(page) - (uint16_t)len;
    memcpy(page->buffer + new_de, data, (size_t)len);
    set_slot_off(page, slot, new_de);
    set_slot_len(page, slot, (uint16_t)len);
    set_data_end(page, new_de);
    return 0;
}

void slotted_page_defrag(SlottedPage *page) {
    int count = (int)slot_count(page);
    if (count == 0) {
        set_data_end(page, PAGE_SIZE);
        return;
    }
    uint8_t *tmp = (uint8_t *)malloc(PAGE_SIZE);
    int *lens = (int *)malloc((size_t)count * sizeof(int));
    if (!tmp || !lens) { free(tmp); free(lens); return; }

    int toff = 0;
    for (int i = 0; i < count; i++) {
        if (slot_is_deleted(page, i)) {
            lens[i] = 0;
        } else {
            int l = (int)slot_len(page, i);
            memcpy(tmp + toff, page->buffer + slot_off(page, i), (size_t)l);
            lens[i] = l;
            toff += l;
        }
    }

    set_data_end(page, PAGE_SIZE);
    toff = 0;
    for (int i = 0; i < count; i++) {
        if (lens[i] == 0) continue;
        uint16_t no = data_end(page) - (uint16_t)lens[i];
        memcpy(page->buffer + no, tmp + toff, (size_t)lens[i]);
        set_slot_off(page, i, no);
        set_slot_len(page, i, (uint16_t)lens[i]);
        set_data_end(page, no);
        toff += lens[i];
    }
    free(tmp);
    free(lens);
}

/* ---- tests ---- */

static int nfail = 0;
#define TEST(n)  do { printf("  %s ... ", n); } while(0)
#define CHECK(c, m) do { if (!(c)) { printf("FAIL: %s\n", m); nfail++; return; } } while(0)

static void test_insert_and_get(void) {
    SlottedPage p;
    slotted_page_init(&p, 42);
    int s = slotted_page_insert(&p, (const uint8_t *)"hello", 5);
    CHECK(s == 0, "first slot");
    int len;
    const uint8_t *d = slotted_page_get(&p, s, &len);
    CHECK(len == 5, "length");
    CHECK(memcmp(d, "hello", 5) == 0, "content");
    CHECK(slotted_page_id(&p) == 42, "page id");
}

static void test_delete(void) {
    SlottedPage p;
    slotted_page_init(&p, 0);
    int s = slotted_page_insert(&p, (const uint8_t *)"hello", 5);
    slotted_page_delete(&p, s);
    int len;
    CHECK(slotted_page_get(&p, s, &len) == NULL, "deleted gone");
}

static void test_update_shrink(void) {
    SlottedPage p;
    slotted_page_init(&p, 0);
    int s = slotted_page_insert(&p, (const uint8_t *)"longer", 6);
    CHECK(slotted_page_update(&p, s, (const uint8_t *)"hi", 2) == 0, "shrink ok");
    int len; const uint8_t *d = slotted_page_get(&p, s, &len);
    CHECK(len == 2, "shrink len"); CHECK(memcmp(d, "hi", 2) == 0, "shrink content");
}

static void test_update_grow(void) {
    SlottedPage p;
    slotted_page_init(&p, 0);
    int s = slotted_page_insert(&p, (const uint8_t *)"hi", 2);
    CHECK(slotted_page_update(&p, s, (const uint8_t *)"longer data", 11) == 0, "grow ok");
    int len; const uint8_t *d = slotted_page_get(&p, s, &len);
    CHECK(len == 11, "grow len"); CHECK(memcmp(d, "longer data", 11) == 0, "grow content");
}

static void test_defrag(void) {
    SlottedPage p;
    slotted_page_init(&p, 0);
    int s0 = slotted_page_insert(&p, (const uint8_t *)"aaa", 3);
    int s1 = slotted_page_insert(&p, (const uint8_t *)"bbb", 3);
    int s2 = slotted_page_insert(&p, (const uint8_t *)"ccc", 3);
    slotted_page_delete(&p, s1);
    slotted_page_defrag(&p);
    int len;
    const uint8_t *d = slotted_page_get(&p, s0, &len);
    CHECK(len == 3 && memcmp(d, "aaa", 3) == 0, "s0 ok");
    CHECK(slotted_page_get(&p, s1, &len) == NULL, "s1 deleted");
    d = slotted_page_get(&p, s2, &len);
    CHECK(len == 3 && memcmp(d, "ccc", 3) == 0, "s2 ok");
}

static void test_page_full(void) {
    SlottedPage p;
    slotted_page_init(&p, 0);
    uint8_t *big = (uint8_t *)malloc(PAGE_SIZE);
    memset(big, 0, PAGE_SIZE);
    CHECK(slotted_page_insert(&p, big, PAGE_SIZE - HEADER_SIZE - SLOT_ENTRY_SIZE) >= 0, "big fits");
    CHECK(slotted_page_insert(&p, (const uint8_t *)"x", 1) < 0, "overflow fails");
    free(big);
}

static void test_utilization(void) {
    SlottedPage p;
    slotted_page_init(&p, 0);
    double u1 = 1.0 - (double)free_space(&p) / (double)PAGE_SIZE;
    CHECK(u1 > 0.0, "empty page util");
    (void)u1;
}

int main(void) {
    printf("Slotted Page tests:\n");
    test_insert_and_get();
    test_delete();
    test_update_shrink();
    test_update_grow();
    test_defrag();
    test_page_full();
    test_utilization();
    if (nfail == 0) printf("All tests PASSED.\n");
    else printf("%d test(s) FAILED.\n", nfail);
    return nfail > 0 ? 1 : 0;
}
