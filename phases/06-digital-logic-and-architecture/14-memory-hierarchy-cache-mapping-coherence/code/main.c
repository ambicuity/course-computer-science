#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>

#define DEFAULT_BLOCK_SIZE 64
#define DEFAULT_NUM_SETS 64
#define DEFAULT_ASSOCIATIVITY 1

typedef struct {
    bool valid;
    bool dirty;
    uint32_t tag;
    unsigned int lru_counter; /* higher = more recently used */
} CacheLine;

typedef struct {
    CacheLine *lines;
    unsigned int num_sets;
    unsigned int associativity;
    unsigned int block_size;
    unsigned int index_bits;
    unsigned int offset_bits;
    unsigned int hits;
    unsigned int misses;
    unsigned int evictions;
    unsigned int writebacks;
    unsigned int access_count;
} Cache;

Cache *cache_init(unsigned int num_sets, unsigned int associativity, unsigned int block_size) {
    Cache *c = malloc(sizeof(Cache));
    c->num_sets = num_sets;
    c->associativity = associativity;
    c->block_size = block_size;
    c->hits = 0;
    c->misses = 0;
    c->evictions = 0;
    c->writebacks = 0;
    c->access_count = 0;

    unsigned int total_lines = num_sets * associativity;
    c->lines = calloc(total_lines, sizeof(CacheLine));

    /* compute bit widths */
    c->offset_bits = 0;
    unsigned int bs = block_size;
    while (bs > 1) { c->offset_bits++; bs >>= 1; }

    c->index_bits = 0;
    unsigned int ns = num_sets;
    while (ns > 1) { c->index_bits++; ns >>= 1; }

    return c;
}

void cache_free(Cache *c) {
    if (c) {
        free(c->lines);
        free(c);
    }
}

static unsigned int get_index(Cache *c, uint32_t address) {
    return (address >> c->offset_bits) & ((1u << c->index_bits) - 1);
}

static uint32_t get_tag(Cache *c, uint32_t address) {
    return address >> (c->offset_bits + c->index_bits);
}

static CacheLine *get_set(Cache *c, unsigned int set_index) {
    return &c->lines[set_index * c->associativity];
}

static int find_line(Cache *c, unsigned int set_index, uint32_t tag) {
    CacheLine *set = get_set(c, set_index);
    for (unsigned int i = 0; i < c->associativity; i++) {
        if (set[i].valid && set[i].tag == tag)
            return (int)i;
    }
    return -1;
}

static int find_victim_lru(Cache *c, unsigned int set_index) {
    CacheLine *set = get_set(c, set_index);
    unsigned int min_lru = set[0].lru_counter;
    int victim = 0;
    for (unsigned int i = 1; i < c->associativity; i++) {
        if (set[i].lru_counter < min_lru) {
            min_lru = set[i].lru_counter;
            victim = (int)i;
        }
    }
    return victim;
}

static int find_empty_line(Cache *c, unsigned int set_index) {
    CacheLine *set = get_set(c, set_index);
    for (unsigned int i = 0; i < c->associativity; i++) {
        if (!set[i].valid)
            return (int)i;
    }
    return -1;
}

bool cache_access(Cache *c, uint32_t address, bool is_write) {
    c->access_count++;
    unsigned int set_index = get_index(c, address);
    uint32_t tag = get_tag(c, address);
    int way = find_line(c, set_index, tag);

    if (way >= 0) {
        /* hit */
        c->hits++;
        CacheLine *line = &get_set(c, set_index)[way];
        line->lru_counter = c->access_count;
        if (is_write)
            line->dirty = true;
        return true;
    }

    /* miss */
    c->misses++;
    int target = find_empty_line(c, set_index);
    if (target < 0) {
        /* evict */
        target = find_victim_lru(c, set_index);
        CacheLine *victim = &get_set(c, set_index)[target];
        if (victim->dirty) {
            c->writebacks++;
        }
        c->evictions++;
    }

    CacheLine *line = &get_set(c, set_index)[target];
    line->valid = true;
    line->dirty = is_write;
    line->tag = tag;
    line->lru_counter = c->access_count;
    return false;
}

void cache_print_stats(Cache *c, const char *label) {
    unsigned int total = c->hits + c->misses;
    double hit_rate = total > 0 ? (100.0 * c->hits / total) : 0.0;
    printf("[%s] accesses=%u hits=%u misses=%u evictions=%u writebacks=%u hit_rate=%.1f%%\n",
           label, total, c->hits, c->misses, c->evictions, c->writebacks, hit_rate);
}

/*
 * Simulate a stride-2K access pattern over a 4 KB working set.
 * With a 4 KB direct-mapped cache (64 sets, 64-byte lines),
 * addresses 2048 bytes apart map to the same set and thrash.
 * With a 2-way set-associative cache, both fit.
 */
void demo_conflict_misses(void) {
    printf("=== Conflict Miss Demo ===\n");
    printf("Pattern: repeatedly access addresses 4096 bytes apart\n");
    printf("Cache: 4 KB total, 64-byte blocks\n\n");

    unsigned int num_sets_dm = 64;  /* 4096 / 64 = 64 sets (direct-mapped) */
    unsigned int num_sets_2w = 32;  /* 4096 / (2 * 64) = 32 sets (2-way) */

    Cache *dm = cache_init(num_sets_dm, 1, DEFAULT_BLOCK_SIZE);
    Cache *tw = cache_init(num_sets_2w, 2, DEFAULT_BLOCK_SIZE);

    /*
     * Address 0 → set (0 >> 6) & 63 = 0
     * Address 4096 → set (4096 >> 6) & 63 = 64 & 63 = 0  (same set!)
     * Direct-mapped: every access to 4096 evicts 0, and vice versa.
     * 2-way: both lines coexist in set 0.
     */
    for (int i = 0; i < 20; i++) {
        cache_access(dm, 0, false);
        cache_access(dm, 4096, false);

        cache_access(tw, 0, false);
        cache_access(tw, 4096, false);
    }

    printf("Direct-mapped (1-way):\n");
    cache_print_stats(dm, "1-way");
    printf("\n2-way set-associative:\n");
    cache_print_stats(tw, "2-way");
    printf("\nDirect-mapped thrashes because addresses 0 and 4096 map to set 0.\n");
    printf("2-way associativity lets both lines coexist.\n\n");

    cache_free(dm);
    cache_free(tw);
}

/*
 * Demonstrate general locality: sequential scan of 1 KB.
 * Both configurations should perform well; associativity matters less
 * when there are no conflict misses.
 */
void demo_sequential_access(void) {
    printf("=== Sequential Access Demo ===\n");
    printf("Pattern: sequential read of 1024 bytes\n");
    printf("Cache: 1 KB total, 64-byte blocks\n\n");

    unsigned int num_sets = 16; /* 1024 / 64 */
    Cache *dm = cache_init(num_sets, 1, DEFAULT_BLOCK_SIZE);
    Cache *fa = cache_init(1, 16, DEFAULT_BLOCK_SIZE); /* fully associative (16 lines in 1 set) */

    for (uint32_t addr = 0; addr < 1024; addr += 4) {
        cache_access(dm, addr, false);
        cache_access(fa, addr, false);
    }

    printf("Direct-mapped (16 sets, 1-way):\n");
    cache_print_stats(dm, "direct");
    printf("\nFully associative (1 set, 16 lines):\n");
    cache_print_stats(fa, "full-assoc");
    printf("\nSequential access has good spatial locality;\n");
    printf("both configurations yield similar results.\n\n");

    cache_free(dm);
    cache_free(fa);
}

/*
 * Demonstrate write-back behavior: repeated writes to the same line
 * should only dirty it once and cause a single writeback on eviction.
 */
void demo_write_back(void) {
    printf("=== Write-Back Demo ===\n");
    printf("Pattern: 10 writes to same address, then access an evicting address\n");
    printf("Cache: 1 KB, direct-mapped, 64-byte blocks\n\n");

    Cache *c = cache_init(16, 1, DEFAULT_BLOCK_SIZE);

    for (int i = 0; i < 10; i++)
        cache_access(c, 0x100, true); /* write to address 0x100 */

    /* access a different address that maps to the same set to force eviction */
    /* 16 sets * 64 bytes = 1024 bytes stride to hit same set */
    cache_access(c, 0x100 + 1024, false);

    cache_print_stats(c, "write-back");
    printf("Only 1 writeback despite 10 writes — line was dirty once, written back on eviction.\n\n");

    cache_free(c);
}

int main(void) {
    printf("========================================\n");
    printf("  Cache Simulator — Mapping & Coherence\n");
    printf("========================================\n\n");

    demo_conflict_misses();
    demo_sequential_access();
    demo_write_back();

    return 0;
}
