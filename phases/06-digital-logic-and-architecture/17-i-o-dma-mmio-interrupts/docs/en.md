# I/O — DMA, MMIO, Interrupts

> A CPU can execute billions of instructions per second, but a disk takes milliseconds to respond. The I/O subsystem bridges that 10⁶× speed gap without wasting CPU cycles.

**Type:** Learn | **Languages:** C | **Prerequisites:** Phase 06 lessons 01–16 | **Time:** ~60 minutes

## Learning Objectives

- Explain why programmed I/O wastes CPU cycles and how interrupts solve that.
- Map device registers into the physical address space (MMIO) and access them via load/store.
- Describe DMA transfers and why the CPU is free during them.
- Build an I/O simulator comparing polling, interrupt, and DMA approaches.

## The Problem

This lesson sits in **Phase 06 — Digital Logic & Computer Architecture**. Without I/O, the CPU is a sealed box — it computes results but can never read input or produce output. Every real system (keyboard, NIC, NVMe, GPU) depends on the I/O mechanisms taught here.

## The Concept

### The I/O Bottleneck

| Device | Latency | Throughput |
|--------|---------|------------|
| CPU register | 1 cycle | — |
| L1 cache | 3–4 cycles | — |
| DRAM | 100–300 cycles | 50 GB/s |
| NVMe SSD | ~10 µs | 7 GB/s |
| HDD | ~10 ms | 200 MB/s |
| Network (1 GbE) | ~50 µs | 125 MB/s |

The CPU is roughly 1000× faster than storage and 1,000,000× faster than disk. If the CPU busy-waits for every I/O operation, it wastes nearly all its time.

### Programmed I/O (Polling)

The simplest approach: the CPU repeatedly reads a device's **status register** until the device signals it is ready, then reads/writes the **data register**.

```
while (!(status_reg & READY_BIT)) { /* busy-wait */ }
data = data_reg;
```

**Problem:** the CPU burns cycles spinning. For a 10 ms disk read at 3 GHz, the CPU wastes 30 million cycles doing nothing.

### Interrupts

Instead of polling, the device asserts an **IRQ (interrupt request)** line when it is ready. The CPU:

1. Completes or pauses the current instruction.
2. Saves the program counter (PC) and status register on the stack.
3. Looks up the **interrupt vector table (IVT)** — an array of handler addresses indexed by IRQ number.
4. Jumps to the **interrupt handler** (ISR — interrupt service routine).
5. The ISR reads data from the device, acknowledges the interrupt, and returns.

```
          Device
            │
            ▼  IRQ line
     ┌──────────────┐
     │  Interrupt    │
     │  Controller   │───► CPU (INTR pin)
     │  (PIC / APIC) │
     └──────────────┘
            ▲
     IRQ0  IRQ1  IRQ2  ...  IRQ15
     Timer  KB   COM1       ...
```

**Interrupt vector table**: maps IRQ numbers to handler addresses. On x86, IVT is at address `0x0000` (real mode) or the IDT at a configurable base (protected mode).

**Interrupt priority**: higher-priority interrupts can preempt lower-priority handlers (**nested interrupts**). The interrupt controller arbitrates when multiple IRQs arrive simultaneously.

**Cost**: saving/restoring context is ~100–500 cycles — negligible compared to the millions of cycles saved by not polling.

### MMIO — Memory-Mapped I/O

Device control, status, and data registers are mapped into the **physical address space**. The CPU accesses them with normal `load`/`store` instructions — no special I/O instructions needed.

```
Physical address space
┌─────────────────────────┐  0xFFFFFFFF
│   Device registers      │  (e.g., UART at 0x10000000)
│   (MMIO region)         │
├─────────────────────────┤
│   DRAM                  │
│                         │
└─────────────────────────┘  0x00000000
```

Example: a UART transmit register at physical address `0x10000000`.

```c
volatile uint32_t *uart_tx = (volatile uint32_t *)0x10000000;
*uart_tx = 'A';  // write character to UART — normal store instruction
```

The `volatile` keyword prevents the compiler from caching the value in a register — every access hits the device.

### DMA — Direct Memory Access

A **DMA controller** transfers data between a device and memory **without CPU involvement**.

```
  CPU ──────── Memory Bus ──────── Memory
                 ▲
                 │
           DMA Controller
                 │
              Device
```

**How it works:**

1. The CPU programs the DMA controller: source address, destination address, byte count.
2. The DMA controller takes over the memory bus for one transfer at a time.
3. When the transfer completes, the DMA controller raises an interrupt.
4. The CPU is free to execute other instructions during the transfer.

**Scatter-gather**: the DMA controller follows a linked list of (address, length) pairs, transferring non-contiguous memory regions in a single operation.

**Bus mastering**: modern PCIe devices have their own DMA engines — they read/write memory directly without a separate DMA controller chip.

**Example** — a 1 MB disk read:
- Programmed I/O: CPU spins for ~10 ms (30M cycles wasted).
- Interrupt-driven: CPU gets an interrupt per sector (512 bytes) — 2048 interrupts for 1 MB.
- DMA: CPU programs one transfer, does other work, gets one interrupt when done.

### Interrupt Coalescing

Modern NICs and NVMe controllers **coalesce** multiple interrupt events into a single interrupt to reduce overhead. Instead of raising an IRQ per packet, the device waits a short time (or until N events accumulate) and raises one IRQ for the batch. Trade-off: latency vs. throughput.

## Build It

The accompanying `code/main.c` simulates three I/O strategies and compares their CPU utilization. It models:

- `InterruptController` with a priority queue of pending IRQs.
- `DMAController` that transfers blocks without CPU involvement.
- `handle_interrupt()` dispatches to the correct handler via a vector table.
- `simulate_io_polling()` vs `simulate_io_interrupt()` — side-by-side CPU cost comparison.
- `simulate_dma_transfer()` — shows the CPU is free during the transfer.

### Step 1: Interrupt Controller

```c
#define MAX_IRQ 16
#define MAX_PENDING 32

typedef struct {
    int irq_num;
    int priority;
} Interrupt;

typedef struct {
    Interrupt pending[MAX_PENDING];
    int count;
    void (*handlers[MAX_IRQ])(int irq);  // interrupt vector table
    int nesting_level;
} InterruptController;
```

The controller maintains a priority queue: higher-priority IRQs are dispatched first. Nesting level tracks whether we are already inside a handler.

### Step 2: DMA Controller

```c
typedef struct {
    int active;
    uint32_t src_addr;
    uint32_t dst_addr;
    int bytes_remaining;
    int bytes_per_cycle;  // transfer rate
    void (*completion_handler)(int irq);
} DMAController;
```

The DMA controller runs on each simulated cycle: it moves `bytes_per_cycle` bytes from source to destination and raises an interrupt when `bytes_remaining` reaches zero.

### Step 3: Simulation

Run 1000-cycle simulations for each strategy, printing how many cycles the CPU spent on I/O vs useful work. See `code/main.c` for the full implementation.

## Use It

Every modern OS and hardware platform uses these mechanisms:

- **Linux**: `/proc/interrupts` shows per-CPU interrupt counts. `request_irq()` registers handlers. `ioremap()` maps MMIO regions. `dma_alloc_coherent()` allocates DMA buffers.
- **NVMe**: uses doorbell registers (MMIO) to submit commands and MSI-X interrupts for completion notification. Data transfers use bus-mastering DMA.
- **PCIe**: all device interaction is through MMIO BARs (Base Address Registers) and DMA. There is no legacy I/O port access on modern systems.

## Read the Source

- Linux `kernel/irq/manage.c` — interrupt request/free infrastructure.
- Linux `drivers/pci/msi.c` — Message Signaled Interrupts (MSI-X).
- Linux `kernel/dma/mapping.c` — DMA mapping API.
- Intel SDM Vol. 3A, Ch. 6 — Interrupt and Exception Handling.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`io_sim` — a self-contained I/O simulator comparing polling, interrupt, and DMA strategies.**

## Exercises

1. **Easy** — Implement the interrupt vector table dispatch. Register three handlers (timer, keyboard, disk) and verify they are called in priority order.

2. **Medium** — Add interrupt coalescing: instead of dispatching every IRQ immediately, accumulate up to 4 IRQs or wait 10 cycles before dispatching. Measure the reduction in context-switch overhead.

3. **Hard** — Model a scatter-gather DMA transfer: the CPU programs a list of (src, len, dst) descriptors. The DMA controller processes them sequentially, raising an interrupt after each descriptor or after the entire list. Compare total CPU involvement vs. single-block DMA.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| IRQ | "interrupt request line" | A hardware signal from a device to the interrupt controller requesting CPU attention. |
| ISR | "interrupt handler" | The function the CPU executes in response to an interrupt; must be short and fast. |
| IVT / IDT | "interrupt vector table" | An array mapping IRQ numbers to handler addresses. IDT is the x86 protected-mode variant. |
| MMIO | "memory-mapped I/O" | Device registers mapped into the physical address space, accessed via normal load/store. |
| DMA | "direct memory access" | A controller that transfers data between device and memory without CPU involvement. |
| Bus mastering | "the device does DMA itself" | A PCIe device with its own DMA engine reads/writes memory directly. |
| Doorbell | "ring the doorbell" | An MMIO register the CPU writes to notify a device that new work is available. |
| Scatter-gather | "SG list" | A DMA technique transferring non-contiguous memory regions via a descriptor chain. |
| MSI-X | "message signaled interrupt" | PCIe interrupts delivered as memory writes instead of dedicated IRQ lines. |

## Further Reading

- *Computer Organization and Design* (Patterson & Hennessy), Ch. 7 — I/O Systems.
- *Linux Device Drivers* (Corbet, Rubini, Kroah-Hartman), Ch. 10 — Interrupt Handling.
- Intel SDM Vol. 3A, Ch. 6 — Interrupt and Exception Handling.
- PCI Express Base Specification, Ch. 6 — MSI-X.
