/* kernel.c — nanos: a bootable mini-kernel for RISC-V 64-bit
 *
 * Phase 07 Capstone
 * Implements: UART driver, bump allocator, process table,
 *             round-robin scheduler, context switch, minimal shell.
 */

#include <stdint.h>
#include <stddef.h>

/* ===================================================================
 *  UART Driver  (NS16550-compatible, MMIO at 0x10000000)
 * =================================================================== */

#define UART_BASE   0x10000000UL
#define UART_THR    (*(volatile uint8_t *)(UART_BASE + 0))  /* transmit hold reg  */
#define UART_RBR    (*(volatile uint8_t *)(UART_BASE + 0))  /* receive buffer reg */
#define UART_LSR    (*(volatile uint8_t *)(UART_BASE + 5))  /* line status reg    */
#define LSR_TX_IDLE (1u << 5)
#define LSR_RX_RDY  (1u << 0)

static void uart_putc(char c)
{
    while (!(UART_LSR & LSR_TX_IDLE))
        ;
    UART_THR = (uint8_t)c;
}

static void uart_puts(const char *s)
{
    while (*s) {
        if (*s == '\n') uart_putc('\r');
        uart_putc(*s++);
    }
}

static int uart_getc(void)
{
    while (!(UART_LSR & LSR_RX_RDY))
        ;
    return UART_RBR;
}

static void put_dec(uint64_t val)
{
    char buf[21];
    int i = 0;
    if (val == 0) { uart_putc('0'); return; }
    while (val > 0) {
        buf[i++] = '0' + (char)(val % 10);
        val /= 10;
    }
    while (--i >= 0)
        uart_putc(buf[i]);
}

/* ===================================================================
 *  Bump Memory Allocator
 * =================================================================== */

extern char _heap_start;
extern char _heap_end;

static char *heap_ptr;

static void kalloc_init(void)
{
    heap_ptr = &_heap_start;
}

static void *kalloc(size_t size)
{
    size = (size + 7) & ~7;          /* 8-byte align */
    if (heap_ptr + size > &_heap_end)
        return NULL;
    void *ptr = heap_ptr;
    heap_ptr += size;
    return ptr;
}

static uint64_t mem_used(void)  { return (uint64_t)(heap_ptr - &_heap_start); }
static uint64_t mem_total(void) { return (uint64_t)(&_heap_end - &_heap_start); }

/* ===================================================================
 *  Process Management
 * =================================================================== */

#define MAX_PROCS   8
#define STACK_SIZE  4096

typedef enum { PROC_UNUSED, PROC_READY, PROC_RUNNING, PROC_EXITED } ProcState;

typedef struct {
    uint64_t    *sp;                       /* saved kernel stack pointer  */
    ProcState    state;
    char         name[16];
    int          pid;
    void       (*entry)(void);             /* initial entry point         */
    uint64_t     stack[STACK_SIZE / 8];    /* kernel stack (4 KiB)        */
} Proc;

static Proc procs[MAX_PROCS];
static int  nprocs   = 0;
static int  current  = -1;

/* Defined in context.s */
extern void context_switch(uint64_t **old_sp, uint64_t *new_sp);

/* Entry trampoline — first time a process runs, context_switch returns here */
static void proc_start(void)
{
    procs[current].entry();
    procs[current].state = PROC_EXITED;
    /* Spin; scheduler will skip exited processes */
    while (1)
        ;
}

static int proc_create(const char *name, void (*fn)(void))
{
    if (nprocs >= MAX_PROCS) return -1;

    Proc *p = &procs[nprocs];
    p->pid   = nprocs;
    p->state = PROC_READY;
    p->entry = fn;

    int i = 0;
    while (name[i] && i < 15) { p->name[i] = name[i]; i++; }
    p->name[i] = '\0';

    /* Build an initial context on the stack so that context_switch
     * "restores" into proc_start with the correct entry function. */
    uint64_t *sp = &p->stack[STACK_SIZE / 8];  /* top of stack */
    sp -= 14;                                   /* 14 callee-saved slots */
    sp[0]  = (uint64_t)proc_start;  /* ra  */
    sp[1]  = 0;  /* s0  */
    sp[2]  = 0;  /* s1  */
    sp[3]  = 0;  /* s2  */
    sp[4]  = 0;  /* s3  */
    sp[5]  = 0;  /* s4  */
    sp[6]  = 0;  /* s5  */
    sp[7]  = 0;  /* s6  */
    sp[8]  = 0;  /* s7  */
    sp[9]  = 0;  /* s8  */
    sp[10] = 0;  /* s9  */
    sp[11] = 0;  /* s10 */
    sp[12] = 0;  /* s11 */
    sp[13] = 0;  /* gp  */
    p->sp = sp;

    nprocs++;
    return p->pid;
}

/* ===================================================================
 *  Round-Robin Scheduler
 * =================================================================== */

static void schedule(void)
{
    while (1) {
        /* Find next ready process (round-robin from current) */
        int next = -1;
        for (int i = 1; i <= nprocs; i++) {
            int idx = (current + i) % nprocs;
            if (procs[idx].state == PROC_READY) {
                next = idx;
                break;
            }
        }

        if (next < 0) {
            uart_puts("nanos: all processes exited.\n");
            return;
        }

        int prev    = current;
        current     = next;
        procs[current].state = PROC_RUNNING;
        if (prev >= 0 && procs[prev].state == PROC_RUNNING)
            procs[prev].state = PROC_READY;

        /* context_switch(&old_sp, new_sp) */
        context_switch(
            prev >= 0 ? &procs[prev].sp : NULL,
            procs[current].sp
        );

        /* When we get control back, demote running to ready */
        if (procs[current].state == PROC_RUNNING)
            procs[current].state = PROC_READY;
    }
}

/* ===================================================================
 *  Dummy background processes
 * =================================================================== */

static volatile uint64_t counters[3];

static void dummy_0(void) { while (1) counters[0]++; }
static void dummy_1(void) { while (1) counters[1]++; }
static void dummy_2(void) { while (1) counters[2]++; }

/* ===================================================================
 *  Simple Shell
 * =================================================================== */

#define CMD_BUF 128

static int str_cmp(const char *a, const char *b)
{
    while (*a && *a == *b) { a++; b++; }
    return (unsigned char)*a - (unsigned char)*b;
}

static int str_prefix(const char *a, const char *b, int n)
{
    while (n-- > 0 && *a && *a == *b) { a++; b++; }
    return n < 0 ? 0 : (unsigned char)*a - (unsigned char)*b;
}

static void readline(char *buf, int max)
{
    int i = 0;
    uart_puts("nanos> ");
    while (i < max - 1) {
        int c = uart_getc();
        if (c == '\r' || c == '\n') { uart_putc('\n'); break; }
        if (c == 127 || c == 8) {               /* backspace */
            if (i > 0) { i--; uart_puts("\b \b"); }
            continue;
        }
        buf[i++] = (char)c;
        uart_putc((char)c);
    }
    buf[i] = '\0';
}

static void cmd_ps(void)
{
    uart_puts("PID  STATE     NAME\n");
    for (int i = 0; i < nprocs; i++) {
        uart_puts("  ");
        put_dec((uint64_t)procs[i].pid);
        uart_puts("   ");
        switch (procs[i].state) {
            case PROC_UNUSED:  uart_puts("UNUSED    "); break;
            case PROC_READY:   uart_puts("READY     "); break;
            case PROC_RUNNING: uart_puts("RUNNING   "); break;
            case PROC_EXITED:  uart_puts("EXITED    "); break;
        }
        uart_puts(procs[i].name);
        uart_putc('\n');
    }
}

static void cmd_meminfo(void)
{
    uart_puts("Memory: ");
    put_dec(mem_used());
    uart_puts(" / ");
    put_dec(mem_total());
    uart_puts(" bytes used\n");
}

static void cmd_help(void)
{
    uart_puts("Commands:\n");
    uart_puts("  echo <text>  - print text\n");
    uart_puts("  help         - show this help\n");
    uart_puts("  ps           - list processes\n");
    uart_puts("  meminfo      - memory usage\n");
    uart_puts("  counters     - background process counters\n");
    uart_puts("  halt         - shut down\n");
}

static void shell(void)
{
    char buf[CMD_BUF];

    uart_puts("\n========================================\n");
    uart_puts("  nanos - a bootable mini-kernel\n");
    uart_puts("  Phase 07 Operating Systems Capstone\n");
    uart_puts("========================================\n\n");

    while (1) {
        readline(buf, CMD_BUF);
        if (buf[0] == '\0') continue;

        if (str_prefix(buf, "echo ", 5) == 0) {
            uart_puts(buf + 5);
            uart_putc('\n');
        } else if (str_cmp(buf, "help") == 0) {
            cmd_help();
        } else if (str_cmp(buf, "ps") == 0) {
            cmd_ps();
        } else if (str_cmp(buf, "meminfo") == 0) {
            cmd_meminfo();
        } else if (str_cmp(buf, "counters") == 0) {
            uart_puts("counters: ");
            put_dec(counters[0]); uart_puts("  ");
            put_dec(counters[1]); uart_puts("  ");
            put_dec(counters[2]); uart_putc('\n');
        } else if (str_cmp(buf, "halt") == 0) {
            uart_puts("Halting.\n");
            *(volatile uint32_t *)0x100000 = 0x5555;   /* QEMU poweroff */
            while (1) ;
        } else {
            uart_puts("Unknown command: ");
            uart_puts(buf);
            uart_putc('\n');
        }
    }
}

/* ===================================================================
 *  Kernel Entry
 * =================================================================== */

void kernel_main(void)
{
    kalloc_init();

    uart_puts("nanos: booting...\n");

    proc_create("idle",    dummy_0);
    proc_create("worker1", dummy_1);
    proc_create("worker2", dummy_2);

    uart_puts("nanos: 3 background processes created\n");

    /* Run shell (it never returns) */
    shell();

    /* unreachable */
    while (1) ;
}
