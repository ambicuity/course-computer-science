#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

#define NUM_VPAGES  64
#define NUM_FRAMES  16
#define PAGE_SIZE   4096

typedef struct {
    bool valid;
    bool dirty;
    bool ref;
    bool cow;
    int  pfn;          /* physical frame number, -1 if not mapped */
    int  swap_offset;  /* swap location, -1 if not swapped */
} PTE;

typedef struct {
    bool    used;
    bool    zeroed;
    bool    ref;
    int     owner_vpn;
    uint8_t data[PAGE_SIZE];
} Frame;

static PTE   page_table[NUM_VPAGES];
static Frame frames[NUM_FRAMES];
static int   frames_used = 0;

/* Stats */
static int stat_faults   = 0;
static int stat_swaps    = 0;
static int stat_copies   = 0;
static int stat_allocs   = 0;

/* ── Frame allocation ─────────────────────────────────── */

static int alloc_frame(void) {
    for (int i = 0; i < NUM_FRAMES; i++) {
        if (!frames[i].used) {
            frames[i].used = true;
            frames[i].zeroed = false;
            frames_used++;
            stat_allocs++;
            return i;
        }
    }
    /* Simple FIFO eviction */
    static int clock_hand = 0;
    for (int attempt = 0; attempt < NUM_FRAMES * 2; attempt++) {
        int i = clock_hand;
        clock_hand = (clock_hand + 1) % NUM_FRAMES;
        if (frames[i].used && !frames[i].ref) {
            /* Evict */
            int vpn = frames[i].owner_vpn;
            if (vpn >= 0 && page_table[vpn].valid) {
                if (page_table[vpn].dirty) {
                    stat_swaps++;
                    printf("  [swap-out] VPN %d → frame %d (dirty)\n", vpn, i);
                }
                page_table[vpn].valid = false;
                page_table[vpn].swap_offset = vpn; /* simplified */
            }
            frames[i].used = true;
            frames[i].zeroed = false;
            frames[i].owner_vpn = -1;
            stat_allocs++;
            return i;
        }
        frames[i].ref = false; /* clear reference bit */
    }
    fprintf(stderr, "No frame available!\n");
    return -1;
}

/* ── Page Fault Handler ──────────────────────────────── */

static void page_fault_handler(int vpn, bool is_write) {
    printf("  [page-fault] VPN %d (write=%d)\n", vpn, is_write);
    stat_faults++;

    if (vpn < 0 || vpn >= NUM_VPAGES) {
        printf("  [segfault] invalid VPN %d\n", vpn);
        return;
    }

    PTE *pte = &page_table[vpn];

    if (pte->swap_offset >= 0 && !pte->valid) {
        /* Swap in */
        int f = alloc_frame();
        printf("  [swap-in]  VPN %d ← swap offset %d → frame %d\n",
               vpn, pte->swap_offset, f);
        stat_swaps++;
        pte->pfn = f;
        pte->valid = true;
        pte->dirty = false;
        frames[f].owner_vpn = vpn;
        frames[f].zeroed = false;
    } else if (pte->cow) {
        /* Copy-on-write */
        int old_pfn = pte->pfn;
        int new_pfn = alloc_frame();
        printf("  [COW] VPN %d: copy frame %d → frame %d\n", vpn, old_pfn, new_pfn);
        stat_copies++;
        memcpy(frames[new_pfn].data, frames[old_pfn].data, PAGE_SIZE);
        pte->pfn = new_pfn;
        pte->cow = false;
        pte->valid = true;
        pte->dirty = true;
        frames[new_pfn].owner_vpn = vpn;
    } else {
        /* Fresh allocation — zero-fill */
        int f = alloc_frame();
        printf("  [alloc]    VPN %d → frame %d (zero-fill)\n", vpn, f);
        memset(frames[f].data, 0, PAGE_SIZE);
        frames[f].zeroed = true;
        frames[f].owner_vpn = vpn;
        pte->pfn = f;
        pte->valid = true;
        pte->dirty = is_write;
    }
    pte->ref = true;
}

/* ── Access simulation ───────────────────────────────── */

static void access_page(int vpn, bool is_write) {
    PTE *pte = &page_table[vpn];
    if (!pte->valid || (is_write && pte->cow)) {
        page_fault_handler(vpn, is_write);
    } else {
        pte->ref = true;
        if (is_write) pte->dirty = true;
        printf("  [hit]      VPN %d → frame %d (write=%d)\n",
               vpn, pte->pfn, is_write);
    }
}

/* ── mmap simulation ─────────────────────────────────── */

static void mmap_simulate(int start_vpn, int num_pages, bool writable) {
    printf("\nmmap: VPN %d-%d (%d pages, writable=%d)\n",
           start_vpn, start_vpn + num_pages - 1, num_pages, writable);
    for (int i = 0; i < num_pages; i++) {
        int vpn = start_vpn + i;
        page_table[vpn].valid = false;
        page_table[vpn].dirty = false;
        page_table[vpn].cow = !writable; /* shared = COW */
        page_table[vpn].pfn = -1;
        page_table[vpn].swap_offset = vpn; /* backed by file offset */
        printf("  [mmap]     VPN %d mapped (swap_offset=%d)\n", vpn, vpn);
    }
}

/* ── fork / COW simulation ───────────────────────────── */

static void cow_simulate(void) {
    printf("\nFork simulation: marking all valid pages as COW\n");
    for (int i = 0; i < NUM_VPAGES; i++) {
        if (page_table[i].valid) {
            page_table[i].cow = true;
            printf("  [COW-setup] VPN %d → frame %d (read-only)\n",
                   i, page_table[i].pfn);
        }
    }
}

/* ── Print stats ──────────────────────────────────────── */

static void print_stats(void) {
    printf("\n=== Paging Statistics ===\n");
    printf("Page faults:      %d\n", stat_faults);
    printf("Frames allocated: %d\n", stat_allocs);
    printf("Frames in use:    %d / %d\n", frames_used, NUM_FRAMES);
    printf("Swap operations:  %d\n", stat_swaps);
    printf("COW copies:       %d\n", stat_copies);
}

static void print_page_table(void) {
    printf("\n=== Page Table ===\n");
    printf("%-5s %-6s %-5s %-5s %-4s %-5s %-5s\n",
           "VPN", "Valid", "PFN", "Dirty", "Ref", "COW", "Swap");
    for (int i = 0; i < NUM_VPAGES; i++) {
        if (page_table[i].valid || page_table[i].swap_offset >= 0) {
            printf("%-5d %-6d %-5d %-5d %-4d %-5d %-5d\n",
                   i, page_table[i].valid, page_table[i].pfn,
                   page_table[i].dirty, page_table[i].ref,
                   page_table[i].cow, page_table[i].swap_offset);
        }
    }
}

/* ── Main ─────────────────────────────────────────────── */

int main(void) {
    memset(page_table, 0, sizeof(page_table));
    for (int i = 0; i < NUM_VPAGES; i++) {
        page_table[i].pfn = -1;
        page_table[i].swap_offset = -1;
    }
    memset(frames, 0, sizeof(frames));
    for (int i = 0; i < NUM_FRAMES; i++)
        frames[i].owner_vpn = -1;

    printf("=== Demand Paging Simulator ===\n");
    printf("Virtual pages: %d, Physical frames: %d, Page size: %d bytes\n\n",
           NUM_VPAGES, NUM_FRAMES, PAGE_SIZE);

    printf("--- Phase 1: Sequential accesses ---\n");
    for (int vpn = 0; vpn < 8; vpn++)
        access_page(vpn, false);

    printf("\n--- Phase 2: Write to pages 2,3,4 ---\n");
    access_page(2, true);
    access_page(3, true);
    access_page(4, true);

    printf("\n--- Phase 3: Fork → COW ---\n");
    cow_simulate();

    printf("\n--- Phase 4: Child writes to page 3 ---\n");
    access_page(3, true);

    printf("\n--- Phase 5: mmap file (VPN 40-47) ---\n");
    mmap_simulate(40, 8, true);
    access_page(42, false);
    access_page(45, true);

    printf("\n--- Phase 6: Access beyond mapped region (page fault) ---\n");
    access_page(50, false);

    print_page_table();
    print_stats();

    return 0;
}
