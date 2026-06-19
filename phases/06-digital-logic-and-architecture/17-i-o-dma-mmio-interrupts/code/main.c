/*
 * I/O — DMA, MMIO, Interrupts
 * Phase 06 — Digital Logic & Computer Architecture
 *
 * Simulates three I/O strategies: programmed I/O (polling),
 * interrupt-driven I/O, and DMA. Compares CPU utilization.
 */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>

/* ── Interrupt Controller ───────────────────────────────────────── */

#define MAX_IRQ       16
#define MAX_PENDING   32
#define MAX_HANDLERS  MAX_IRQ

typedef struct {
    int irq_num;
    int priority;       /* higher = more urgent */
} Interrupt;

typedef struct {
    Interrupt pending[MAX_PENDING];
    int count;
    void (*handlers[MAX_HANDLERS])(int irq);
    int nesting_level;
} InterruptController;

static void ic_init(InterruptController *ic) {
    memset(ic, 0, sizeof(*ic));
}

static void ic_register_handler(InterruptController *ic, int irq,
                                 void (*handler)(int irq)) {
    if (irq >= 0 && irq < MAX_HANDLERS)
        ic->handlers[irq] = handler;
}

/* Insert IRQ into priority queue (simple insertion sort). */
static void ic_raise(InterruptController *ic, int irq, int priority) {
    if (ic->count >= MAX_PENDING) {
        printf("  [IRQ %d dropped — pending queue full]\n", irq);
        return;
    }
    int pos = ic->count;
    while (pos > 0 && ic->pending[pos - 1].priority < priority) {
        ic->pending[pos] = ic->pending[pos - 1];
        pos--;
    }
    ic->pending[pos].irq_num = irq;
    ic->pending[pos].priority = priority;
    ic->count++;
}

/* Dispatch the highest-priority pending interrupt. Returns 1 if dispatched. */
static int ic_dispatch(InterruptController *ic) {
    if (ic->count == 0)
        return 0;
    Interrupt intr = ic->pending[0];
    /* shift remaining entries */
    for (int i = 1; i < ic->count; i++)
        ic->pending[i - 1] = ic->pending[i];
    ic->count--;

    if (ic->handlers[intr.irq_num]) {
        ic->nesting_level++;
        printf("  [IRQ %d dispatched (priority %d), nesting=%d]\n",
               intr.irq_num, intr.priority, ic->nesting_level);
        ic->handlers[intr.irq_num](intr.irq_num);
        ic->nesting_level--;
    } else {
        printf("  [IRQ %d — no handler registered]\n", intr.irq_num);
    }
    return 1;
}

/* ── DMA Controller ─────────────────────────────────────────────── */

typedef struct {
    int active;
    uint32_t src_addr;
    uint32_t dst_addr;
    int bytes_remaining;
    int bytes_per_cycle;
    void (*completion_handler)(int irq);
} DMAController;

static void dma_init(DMAController *dma) {
    memset(dma, 0, sizeof(*dma));
}

static void dma_start(DMAController *dma, uint32_t src, uint32_t dst,
                       int bytes, int rate,
                       void (*on_complete)(int irq)) {
    dma->active = 1;
    dma->src_addr = src;
    dma->dst_addr = dst;
    dma->bytes_remaining = bytes;
    dma->bytes_per_cycle = rate;
    dma->completion_handler = on_complete;
    printf("  DMA started: %d bytes from 0x%08X to 0x%08X (%d B/cycle)\n",
           bytes, src, dst, rate);
}

/* Returns number of bytes transferred this cycle (0 if idle). */
static int dma_tick(DMAController *dma) {
    if (!dma->active)
        return 0;
    int transferred = dma->bytes_per_cycle;
    if (transferred > dma->bytes_remaining)
        transferred = dma->bytes_remaining;
    dma->src_addr += transferred;
    dma->dst_addr += transferred;
    dma->bytes_remaining -= transferred;
    if (dma->bytes_remaining <= 0) {
        dma->active = 0;
        printf("  DMA transfer complete\n");
        if (dma->completion_handler)
            dma->completion_handler(3);  /* IRQ 3 = DMA complete */
    }
    return transferred;
}

/* ── Simulated Device & Handlers ────────────────────────────────── */

#define TOTAL_IO_BYTES    4096
#define SECTOR_SIZE       512
#define SIM_CYCLES        100

static int g_cycles_used;  /* CPU cycles consumed by I/O */

static void timer_handler(int irq) {
    (void)irq;
    printf("    → Timer tick\n");
    g_cycles_used += 10;
}

static void keyboard_handler(int irq) {
    (void)irq;
    printf("    → Key pressed: 'A'\n");
    g_cycles_used += 50;
}

static void disk_handler(int irq) {
    (void)irq;
    printf("    → Disk sector transferred\n");
    g_cycles_used += 20;
}

static void dma_complete_handler(int irq) {
    (void)irq;
    printf("    → DMA completion: all %d bytes received\n", TOTAL_IO_BYTES);
    g_cycles_used += 30;
}

/* ── Simulation 1: Programmed I/O (Polling) ─────────────────────── */

static void simulate_io_polling(void) {
    printf("\n═══ Simulation 1: Programmed I/O (Polling) ═══\n");
    printf("Reading %d bytes in %d-byte sectors via polling...\n\n",
           TOTAL_IO_BYTES, SECTOR_SIZE);

    int cpu_useful_cycles = 0;
    g_cycles_used = 0;
    int device_ready_counter = 0;
    int bytes_transferred = 0;
    int cycles_device_busy = 8;  /* device takes 8 cycles per sector */

    for (int cycle = 0; cycle < SIM_CYCLES && bytes_transferred < TOTAL_IO_BYTES; cycle++) {
        /* CPU polls status register every cycle */
        device_ready_counter++;
        g_cycles_used++;  /* cost of reading status register */

        if (device_ready_counter >= cycles_device_busy) {
            /* Device ready — CPU reads data register */
            bytes_transferred += SECTOR_SIZE;
            g_cycles_used += 5;  /* cost of reading data */
            device_ready_counter = 0;
            if (bytes_transferred % (SECTOR_SIZE * 4) == 0)
                printf("  Cycle %3d: sector transferred, total %d bytes\n",
                       cycle, bytes_transferred);
        } else {
            /* CPU spins — does no useful work */
            cpu_useful_cycles++;
        }
    }

    printf("\n  Results:\n");
    printf("    CPU cycles on I/O (polling): %d\n", g_cycles_used);
    printf("    CPU cycles on useful work:   %d\n", cpu_useful_cycles);
    printf("    I/O CPU overhead:            %.1f%%\n",
           100.0 * g_cycles_used / (g_cycles_used + cpu_useful_cycles));
}

/* ── Simulation 2: Interrupt-Driven I/O ─────────────────────────── */

static void simulate_io_interrupt(void) {
    printf("\n═══ Simulation 2: Interrupt-Driven I/O ═══\n");

    InterruptController ic;
    ic_init(&ic);
    ic_register_handler(&ic, 0, timer_handler);
    ic_register_handler(&ic, 1, keyboard_handler);
    ic_register_handler(&ic, 2, disk_handler);

    int cpu_useful_cycles = 0;
    g_cycles_used = 0;
    int sectors_needed = TOTAL_IO_BYTES / SECTOR_SIZE;
    int sector_counter = 0;

    printf("Simulating %d sectors with interrupt-driven I/O...\n\n", sectors_needed);

    for (int cycle = 0; cycle < SIM_CYCLES && sector_counter < sectors_needed; cycle++) {
        /* Device raises IRQ every 8 cycles (simulating sector ready) */
        if (cycle > 0 && cycle % 8 == 0 && sector_counter < sectors_needed) {
            ic_raise(&ic, 2, 5);  /* disk IRQ, priority 5 */
            sector_counter++;
        }

        /* Timer fires every 25 cycles */
        if (cycle > 0 && cycle % 25 == 0)
            ic_raise(&ic, 0, 2);  /* timer IRQ, priority 2 */

        /* Keyboard at cycle 30 */
        if (cycle == 30)
            ic_raise(&ic, 1, 3);  /* keyboard IRQ, priority 3 */

        /* CPU does useful work while waiting */
        cpu_useful_cycles += 5;

        /* Dispatch pending interrupts (cost: ~10 cycles per dispatch) */
        while (ic_dispatch(&ic))
            g_cycles_used += 10;  /* context save/restore overhead */
    }

    printf("\n  Results:\n");
    printf("    CPU cycles on I/O (interrupts): %d\n", g_cycles_used);
    printf("    CPU cycles on useful work:      %d\n", cpu_useful_cycles);
    printf("    I/O CPU overhead:               %.1f%%\n",
           100.0 * g_cycles_used / (g_cycles_used + cpu_useful_cycles));
}

/* ── Simulation 3: DMA Transfer ─────────────────────────────────── */

static void simulate_dma_transfer(void) {
    printf("\n═══ Simulation 3: DMA Transfer ═══\n");

    InterruptController ic;
    DMAController dma;
    ic_init(&ic);
    dma_init(&dma);
    ic_register_handler(&ic, 3, dma_complete_handler);

    int cpu_useful_cycles = 0;
    g_cycles_used = 0;

    printf("Simulating DMA transfer of %d bytes...\n\n", TOTAL_IO_BYTES);

    /* CPU programs DMA controller (cost: ~20 cycles) */
    g_cycles_used += 20;
    dma_start(&dma, 0x10000000, 0x20000000, TOTAL_IO_BYTES, 256, dma_complete_handler);

    for (int cycle = 0; cycle < SIM_CYCLES; cycle++) {
        /* DMA runs independently — CPU does useful work */
        cpu_useful_cycles += 8;
        dma_tick(&dma);

        /* Timer interrupt at cycle 50 */
        if (cycle == 50)
            ic_raise(&ic, 0, 2);

        /* Dispatch any pending interrupts */
        while (ic_dispatch(&ic))
            g_cycles_used += 10;

        if (!dma.active && ic.count == 0)
            break;
    }

    printf("\n  Results:\n");
    printf("    CPU cycles on I/O (DMA):    %d\n", g_cycles_used);
    printf("    CPU cycles on useful work:  %d\n", cpu_useful_cycles);
    printf("    I/O CPU overhead:           %.1f%%\n",
           100.0 * g_cycles_used / (g_cycles_used + cpu_useful_cycles));
}

/* ── Main ───────────────────────────────────────────────────────── */

int main(void) {
    printf("╔══════════════════════════════════════════════════════════╗\n");
    printf("║   I/O Simulation: Polling vs Interrupts vs DMA          ║\n");
    printf("╚══════════════════════════════════════════════════════════╝\n");

    simulate_io_polling();
    simulate_io_interrupt();
    simulate_dma_transfer();

    printf("\n═══ Summary ═══\n");
    printf("Polling:    CPU wastes cycles spinning on status register.\n");
    printf("Interrupts: CPU works between IRQs; context switch ~10 cycles.\n");
    printf("DMA:        CPU programs transfer then is free; one IRQ at end.\n");

    return 0;
}
