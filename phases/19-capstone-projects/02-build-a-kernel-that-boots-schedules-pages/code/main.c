// Build a Kernel That Boots, Schedules, Pages
// Run: gcc -o kernel main.c && ./kernel
//
// Architecture:
//   Boot entry → Trap vector → Memory init (paging) → Scheduler (round-robin)
//
// This implements a simulated kernel with round-robin scheduling and Sv39-style
// page table management. Runs on host OS for educational purposes.
// For bare-metal RISC-V, see main.s and the assembly boot code.

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

// =============================================================================
// Step 1: Page Table Setup (Sv39-style)
// =============================================================================

#define PAGE_SIZE   4096
#define PT_ENTRIES  512

// Page table entry flags
#define PTE_V   (1 << 0)  // Valid
#define PTE_R   (1 << 1)  // Readable
#define PTE_W   (1 << 2)  // Writable
#define PTE_X   (1 << 3)  // Executable

typedef uint64_t pte_t;

// Simplified page table pool (in real kernel, uses physical page allocator)
static pte_t page_table_pool[4096 * 16] __attribute__((aligned(PAGE_SIZE)));
static int pool_offset = 0;

static pte_t *alloc_page_table(void) {
    pte_t *table = &page_table_pool[pool_offset];
    pool_offset += PT_ENTRIES;
    for (int i = 0; i < PT_ENTRIES; i++) {
        table[i] = 0;
    }
    return table;
}

static uint64_t pa_to_ppn(void *pa) {
    return ((uint64_t)pa) >> 12;
}

// Map a virtual address to a physical address using 3-level page table
void map_page(pte_t *root, uint64_t va, uint64_t pa, uint64_t flags) {
    uint64_t vpn2 = (va >> 30) & 0x1FF;
    uint64_t vpn1 = (va >> 21) & 0x1FF;
    uint64_t vpn0 = (va >> 12) & 0x1FF;

    // Level 2
    if (!(root[vpn2] & PTE_V)) {
        pte_t *child = alloc_page_table();
        root[vpn2] = (pa_to_ppn(child) << 10) | PTE_V;
    }
    pte_t *l1 = (pte_t *)((root[vpn2] >> 10) << 12);

    // Level 1
    if (!(l1[vpn1] & PTE_V)) {
        pte_t *child = alloc_page_table();
        l1[vpn1] = (pa_to_ppn(child) << 10) | PTE_V;
    }
    pte_t *l0 = (pte_t *)((l1[vpn1] >> 10) << 12);

    // Level 0: leaf entry
    l0[vpn0] = (pa_to_ppn((void *)pa) << 10) | flags | PTE_V;
}

// Initialize kernel page tables
pte_t *vm_init(void) {
    pte_t *kernel_page_table = alloc_page_table();

    // Identity-map first 2MB of physical memory
    for (uint64_t pa = 0; pa < 2 * 1024 * 1024; pa += PAGE_SIZE) {
        map_page(kernel_page_table, pa, pa, PTE_R | PTE_W | PTE_X);
    }

    printf("vm: Sv39 page table initialized, identity-mapped 2MB\n");
    return kernel_page_table;
}

// =============================================================================
// Step 2: Round-Robin Scheduler
// =============================================================================

#define MAX_TASKS   8
#define STACK_SIZE  4096

typedef enum { TASK_UNUSED, TASK_READY, TASK_RUNNING, TASK_BLOCKED } task_state_t;

typedef struct {
    uint64_t regs[32];     // Saved registers
    uint64_t stack[STACK_SIZE / 8];
    task_state_t state;
    int id;
    const char *name;
    void (*entry)(void);
} task_t;

static task_t tasks[MAX_TASKS];
static int current_task = 0;
static int task_count = 0;

int task_create(const char *name, void (*entry)(void)) {
    if (task_count >= MAX_TASKS) return -1;

    task_t *t = &tasks[task_count];
    t->id = task_count;
    t->state = TASK_READY;
    t->name = name;
    t->entry = entry;
    memset(t->regs, 0, sizeof(t->regs));

    task_count++;
    return t->id;
}

static int scheduler_pick(void) {
    for (int i = 1; i <= task_count; i++) {
        int idx = (current_task + i) % task_count;
        if (tasks[idx].state == TASK_READY) {
            return idx;
        }
    }
    return current_task;
}

void schedule(void) {
    int next = scheduler_pick();
    if (next == current_task) return;

    tasks[current_task].state = TASK_READY;
    tasks[next].state = TASK_RUNNING;

    printf("  [sched] switch: task %d (%s) -> task %d (%s)\n",
           current_task, tasks[current_task].name,
           next, tasks[next].name);

    current_task = next;
}

void timer_tick(void) {
    schedule();
}

void task_exit(void) {
    tasks[current_task].state = TASK_UNUSED;
    printf("  [sched] task %d (%s) exited\n", current_task, tasks[current_task].name);
    schedule();
}

// =============================================================================
// Step 3: Trap Handler
// =============================================================================

typedef struct {
    uint64_t sepc;
    uint64_t scause;
    uint64_t stval;
} trap_frame_t;

void trap_handler(trap_frame_t *tf) {
    printf("  [trap] sepc=0x%lx scause=0x%lx stval=0x%lx\n",
           tf->sepc, tf->scause, tf->stval);

    if (tf->scause == 5) {
        // Timer interrupt
        timer_tick();
    }
}

// =============================================================================
// Step 4: Kernel Main
// =============================================================================

// Simulated task functions
void task_a(void) {
    for (int i = 0; i < 3; i++) {
        printf("  [task_a] running iteration %d\n", i);
    }
}

void task_b(void) {
    for (int i = 0; i < 3; i++) {
        printf("  [task_b] running iteration %d\n", i);
    }
}

void task_c(void) {
    for (int i = 0; i < 3; i++) {
        printf("  [task_c] running iteration %d\n", i);
    }
}

int kmain(void) {
    printf("=== Kernel Boot ===\n");
    printf("kernel: boot ok\n");

    // Initialize virtual memory
    pte_t *kernel_pt = vm_init();

    // Create tasks
    task_create("task_a", task_a);
    task_create("task_b", task_b);
    task_create("task_c", task_c);
    printf("sched: created %d tasks\n", task_count);

    // Simulate scheduling rounds
    printf("\n=== Scheduler Simulation ===\n");
    for (int tick = 0; tick < 9; tick++) {
        printf("\n--- Tick %d ---\n", tick);
        tasks[current_task].entry();
        timer_tick();
    }

    printf("\nkernel: simulation complete\n");
    return 0;
}

int main(void) {
    return kmain();
}
