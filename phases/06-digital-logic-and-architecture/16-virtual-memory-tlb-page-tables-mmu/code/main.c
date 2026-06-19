/*
 * Virtual Memory — TLB, Page Tables, MMU
 * Phase 06 — Digital Logic & Computer Architecture
 *
 * A self-contained virtual memory simulator with:
 *   - 2-level page table (directory + page tables)
 *   - Configurable TLB associativity (direct-mapped, N-way, fully-associative)
 *   - LRU page replacement
 *   - Access trace simulation with statistics
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ── Constants ─────────────────────────────────────────────────────── */

#define PAGE_SIZE       4096
#define OFFSET_BITS     12
#define OFFSET_MASK     0x00000FFF

#define DIR_BITS        10
#define TABLE_BITS      10
#define DIR_ENTRIES     (1 << DIR_BITS)   /* 1024 */
#define TABLE_ENTRIES   (1 << TABLE_BITS) /* 1024 */

#define TLB_SIZE        64
#define NUM_FRAMES      256

/* ── Page Table ────────────────────────────────────────────────────── */

typedef struct {
    int valid;
    int dirty;
    int referenced;
    int frame_number;   /* -1 if invalid */
} PageTableEntry;

typedef struct {
    PageTableEntry *directory[DIR_ENTRIES];
} PageTable;

static void pt_init(PageTable *pt) {
    memset(pt->directory, 0, sizeof(pt->directory));
}

static void pt_destroy(PageTable *pt) {
    for (int i = 0; i < DIR_ENTRIES; i++)
        free(pt->directory[i]);
}

/* Extract directory and table indices from a VPN. */
static inline unsigned int dir_index(unsigned int vpn) {
    return (vpn >> TABLE_BITS) & (DIR_ENTRIES - 1);
}
static inline unsigned int tbl_index(unsigned int vpn) {
    return vpn & (TABLE_ENTRIES - 1);
}

/* Look up a VPN in the page table.  Returns pointer to the entry, or NULL
   if the directory slot or table slot is not allocated / not valid. */
static PageTableEntry *pt_lookup(PageTable *pt, unsigned int vpn) {
    unsigned int di = dir_index(vpn);
    if (pt->directory[di] == NULL)
        return NULL;
    unsigned int ti = tbl_index(vpn);
    PageTableEntry *e = &pt->directory[di][ti];
    if (!e->valid)
        return NULL;
    return e;
}

/* Insert a VPN → PFN mapping into the page table. */
static PageTableEntry *pt_insert(PageTable *pt, unsigned int vpn, int pfn) {
    unsigned int di = dir_index(vpn);
    if (pt->directory[di] == NULL) {
        pt->directory[di] = calloc(TABLE_ENTRIES, sizeof(PageTableEntry));
        for (int i = 0; i < TABLE_ENTRIES; i++)
            pt->directory[di][i].frame_number = -1;
    }
    unsigned int ti = tbl_index(vpn);
    PageTableEntry *e = &pt->directory[di][ti];
    e->valid = 1;
    e->dirty = 0;
    e->referenced = 1;
    e->frame_number = pfn;
    return e;
}

/* ── TLB ───────────────────────────────────────────────────────────── */

typedef struct {
    int valid;
    unsigned int vpn;
    int pfn;
    int last_used;
} TLBEntry;

typedef struct {
    TLBEntry entries[TLB_SIZE];
    int associativity;          /* 1 = direct-mapped; TLB_SIZE = fully-assoc */
    int time_counter;
    int hits;
    int misses;
} TLB;

static void tlb_init(TLB *tlb, int associativity) {
    memset(tlb->entries, 0, sizeof(tlb->entries));
    for (int i = 0; i < TLB_SIZE; i++)
        tlb->entries[i].pfn = -1;
    tlb->associativity = associativity;
    tlb->time_counter = 0;
    tlb->hits = 0;
    tlb->misses = 0;
}

/* Number of sets in the TLB. */
static inline int tlb_num_sets(TLB *tlb) {
    return TLB_SIZE / tlb->associativity;
}

/* Look up VPN in TLB.  Returns PFN on hit, -1 on miss. */
static int tlb_lookup(TLB *tlb, unsigned int vpn) {
    int nsets = tlb_num_sets(tlb);
    int set = (int)(vpn % (unsigned)nsets);
    int base = set * tlb->associativity;

    for (int i = 0; i < tlb->associativity; i++) {
        TLBEntry *e = &tlb->entries[base + i];
        if (e->valid && e->vpn == vpn) {
            tlb->hits++;
            tlb->time_counter++;
            e->last_used = tlb->time_counter;
            return e->pfn;
        }
    }
    tlb->misses++;
    return -1;
}

/* Insert VPN → PFN into TLB, evicting LRU entry in the set if full. */
static void tlb_insert(TLB *tlb, unsigned int vpn, int pfn) {
    int nsets = tlb_num_sets(tlb);
    int set = (int)(vpn % (unsigned)nsets);
    int base = set * tlb->associativity;

    /* Look for an invalid (empty) slot first. */
    int victim = -1;
    int min_used = tlb->time_counter + 1;
    for (int i = 0; i < tlb->associativity; i++) {
        TLBEntry *e = &tlb->entries[base + i];
        if (!e->valid) {
            victim = i;
            break;
        }
        if (e->last_used < min_used) {
            min_used = e->last_used;
            victim = i;
        }
    }

    TLBEntry *e = &tlb->entries[base + victim];
    e->valid = 1;
    e->vpn = vpn;
    e->pfn = pfn;
    tlb->time_counter++;
    e->last_used = tlb->time_counter;
}

/* Invalidate all TLB entries matching a given PFN (on eviction). */
static void tlb_invalidate_pfn(TLB *tlb, int pfn) {
    for (int i = 0; i < TLB_SIZE; i++) {
        if (tlb->entries[i].valid && tlb->entries[i].pfn == pfn)
            tlb->entries[i].valid = 0;
    }
}

/* ── Physical Memory & LRU Replacement ─────────────────────────────── */

/* Track which VPN (if any) occupies each physical frame, and when it
   was last used.  -1 means the frame is free. */
static int frame_owner[NUM_FRAMES];     /* VPN or -1 */
static int frame_dirty[NUM_FRAMES];
static int frame_last_used[NUM_FRAMES];
static int frame_clock = 0;
static int frames_used = 0;

static void physmem_init(void) {
    for (int i = 0; i < NUM_FRAMES; i++) {
        frame_owner[i] = -1;
        frame_dirty[i] = 0;
        frame_last_used[i] = 0;
    }
    frames_used = 0;
}

/* Allocate a free frame, or evict using LRU.  Returns frame index. */
static int alloc_frame(PageTable *pt, TLB *tlb, unsigned int vpn, int is_write) {
    frame_clock++;

    /* Try to find a free frame. */
    for (int i = 0; i < NUM_FRAMES; i++) {
        if (frame_owner[i] == -1) {
            frame_owner[i] = (int)vpn;
            frame_dirty[i] = is_write;
            frame_last_used[i] = frame_clock;
            frames_used++;
            return i;
        }
    }

    /* All frames in use — LRU eviction. */
    int victim = 0;
    for (int i = 1; i < NUM_FRAMES; i++) {
        if (frame_last_used[i] < frame_last_used[victim])
            victim = i;
    }

    unsigned int old_vpn = (unsigned)frame_owner[victim];
    /* Invalidate old page table entry. */
    PageTableEntry *old = pt_lookup(pt, old_vpn);
    if (old) {
        old->valid = 0;
        old->frame_number = -1;
    }
    /* Invalidate TLB entry for evicted page. */
    tlb_invalidate_pfn(tlb, victim);

    frame_owner[victim] = (int)vpn;
    frame_dirty[victim] = is_write;
    frame_last_used[victim] = frame_clock;
    return victim;
}

/* ── MMU Translate ─────────────────────────────────────────────────── */

typedef struct {
    int total_accesses;
    int tlb_hits;
    int tlb_misses;
    int page_faults;
    int writes;
} Stats;

static void stats_init(Stats *s) { memset(s, 0, sizeof(*s)); }

/* Translate a virtual address.  Returns physical address, or -1 on failure.
   `is_write` controls whether the dirty bit is set. */
static unsigned int mmu_translate(TLB *tlb, PageTable *pt,
                                   unsigned int vaddr, int is_write,
                                   Stats *stats)
{
    unsigned int vpn = vaddr >> OFFSET_BITS;
    unsigned int offset = vaddr & OFFSET_MASK;

    stats->total_accesses++;
    if (is_write) stats->writes++;

    /* 1. TLB lookup */
    int pfn = tlb_lookup(tlb, vpn);
    if (pfn >= 0) {
        stats->tlb_hits++;
        /* TLB hit — mark referenced + dirty. */
        PageTableEntry *e = pt_lookup(pt, vpn);
        if (e) {
            e->referenced = 1;
            if (is_write) e->dirty = 1;
            frame_last_used[pfn] = ++frame_clock;
        }
        return ((unsigned int)pfn << OFFSET_BITS) | offset;
    }

    /* 2. TLB miss — walk page table. */
    stats->tlb_misses++;
    PageTableEntry *e = pt_lookup(pt, vpn);
    if (e != NULL) {
        /* Page table hit — insert into TLB. */
        tlb_insert(tlb, vpn, e->frame_number);
        e->referenced = 1;
        if (is_write) e->dirty = 1;
        frame_last_used[e->frame_number] = ++frame_clock;
        return ((unsigned int)e->frame_number << OFFSET_BITS) | offset;
    }

    /* 3. Page fault — allocate a frame, create mapping. */
    stats->page_faults++;
    int frame = alloc_frame(pt, tlb, vpn, is_write);
    e = pt_insert(pt, vpn, frame);
    tlb_insert(tlb, vpn, frame);
    return ((unsigned int)frame << OFFSET_BITS) | offset;
}

/* ── Simulation ────────────────────────────────────────────────────── */

/* Run a trace of virtual addresses and collect statistics.
   `modes`: 'r' = read, 'w' = write (one per address). */
static void simulate_accesses(PageTable *pt, TLB *tlb,
                               unsigned int *addrs, char *modes, int count,
                               Stats *stats)
{
    stats_init(stats);
    for (int i = 0; i < count; i++) {
        int is_write = (modes[i] == 'w');
        unsigned int pa = mmu_translate(tlb, pt, addrs[i], is_write, stats);
        (void)pa; /* result used implicitly for side effects */
    }
}

/* ── Access Pattern Generators ─────────────────────────────────────── */

/* Sequential: access pages 0..n-1 in order (read). */
static int gen_sequential(unsigned int *addrs, char *modes, int n) {
    for (int i = 0; i < n; i++) {
        addrs[i] = (unsigned int)i * PAGE_SIZE + (i & OFFSET_MASK);
        modes[i] = 'r';
    }
    return n;
}

/* Random: n random virtual addresses (mixed read/write). */
static int gen_random(unsigned int *addrs, char *modes, int n) {
    for (int i = 0; i < n; i++) {
        addrs[i] = (unsigned int)(rand() % (4096 * PAGE_SIZE));
        modes[i] = (rand() % 4 == 0) ? 'w' : 'r';
    }
    return n;
}

/* Working set: repeatedly access a small set of pages, then stride
   to a new set (simulates a loop over a small array). */
static int gen_working_set(unsigned int *addrs, char *modes, int n) {
    int ws = 8; /* working set size in pages */
    for (int i = 0; i < n; i++) {
        int page = (i % ws) + (i / (ws * 10)) * ws;
        addrs[i] = (unsigned int)page * PAGE_SIZE + 64;
        modes[i] = (i % 5 == 0) ? 'w' : 'r';
    }
    return n;
}

/* ── Reporting ─────────────────────────────────────────────────────── */

static void print_stats(const char *label, Stats *s, TLB *tlb) {
    printf("=== %s ===\n", label);
    printf("  Total accesses:  %d\n", s->total_accesses);
    printf("  TLB hits:        %d  (%.1f%%)\n", s->tlb_hits,
           100.0 * s->tlb_hits / s->total_accesses);
    printf("  TLB misses:      %d  (%.1f%%)\n", s->tlb_misses,
           100.0 * s->tlb_misses / s->total_accesses);
    printf("  Page faults:     %d  (%.2f%%)\n", s->page_faults,
           100.0 * s->page_faults / s->total_accesses);
    printf("  Writes:          %d\n", s->writes);
    printf("  TLB hit rate:    %.1f%%\n\n",
           100.0 * tlb->hits / (tlb->hits + tlb->misses));
}

/* ── Main ──────────────────────────────────────────────────────────── */

#define TRACE_LEN 512

int main(void) {
    unsigned int addrs[TRACE_LEN];
    char modes[TRACE_LEN];
    int n;

    printf("Virtual Memory Simulator\n");
    printf("  Page size:      %d bytes\n", PAGE_SIZE);
    printf("  Physical frames: %d\n", NUM_FRAMES);
    printf("  TLB entries:    %d\n\n", TLB_SIZE);

    /* ---------- Demo 1: Sequential access (read) ------------------- */
    PageTable pt1;
    TLB tlb1;
    Stats stats1;

    pt_init(&pt1);
    tlb_init(&tlb1, 4); /* 4-way set-associative */
    physmem_init();

    n = gen_sequential(addrs, modes, 256);
    simulate_accesses(&pt1, &tlb1, addrs, modes, n, &stats1);
    print_stats("Sequential access (256 pages, 4-way TLB)", &stats1, &tlb1);

    pt_destroy(&pt1);

    /* ---------- Demo 2: Random access ------------------------------ */
    PageTable pt2;
    TLB tlb2;
    Stats stats2;

    pt_init(&pt2);
    tlb_init(&tlb2, 4);
    physmem_init();

    srand(42);
    n = gen_random(addrs, modes, TRACE_LEN);
    simulate_accesses(&pt2, &tlb2, addrs, modes, n, &stats2);
    print_stats("Random access (512 accesses, 4-way TLB)", &stats2, &tlb2);

    pt_destroy(&pt2);

    /* ---------- Demo 3: Working set access ------------------------- */
    PageTable pt3;
    TLB tlb3;
    Stats stats3;

    pt_init(&pt3);
    tlb_init(&tlb3, 4);
    physmem_init();

    n = gen_working_set(addrs, modes, TRACE_LEN);
    simulate_accesses(&pt3, &tlb3, addrs, modes, n, &stats3);
    print_stats("Working-set access (512 accesses, 4-way TLB)", &stats3, &tlb3);

    pt_destroy(&pt3);

    /* ---------- Demo 4: TLB associativity comparison --------------- */
    printf("=== TLB Associativity Comparison (random trace, 512 accesses) ===\n");
    const char *labels[] = {"Direct-mapped", "2-way", "4-way", "8-way", "Fully-assoc"};
    int assocs[] = {1, 2, 4, 8, TLB_SIZE};
    for (int a = 0; a < 5; a++) {
        PageTable pt;
        TLB tlb;
        Stats st;
        pt_init(&pt);
        tlb_init(&tlb, assocs[a]);
        physmem_init();
        srand(42);
        n = gen_random(addrs, modes, TRACE_LEN);
        simulate_accesses(&pt, &tlb, addrs, modes, n, &st);
        printf("  %-15s  TLB hit rate: %5.1f%%  page faults: %d\n",
               labels[a],
               100.0 * tlb.hits / (tlb.hits + tlb.misses),
               st.page_faults);
        pt_destroy(&pt);
    }
    printf("\n");

    return 0;
}
