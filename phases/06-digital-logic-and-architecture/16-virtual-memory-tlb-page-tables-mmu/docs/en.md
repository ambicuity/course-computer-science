# Virtual Memory — TLB, Page Tables, MMU

> Every process believes it owns the entire address space. The MMU, page tables, and TLB make that illusion real.

**Type:** Learn | **Languages:** C | **Prerequisites:** Phase 06 lessons 01–15 | **Time:** ~75 minutes

## Learning Objectives

- Explain how virtual memory isolates processes and multiplexes physical DRAM.
- Translate a virtual address through a multi-level page table.
- Predict TLB hit/miss behavior and estimate page fault cost.
- Build a virtual memory simulator in C and measure page fault rates.

## The Problem

This lesson sits in **Phase 06 — Digital Logic & Computer Architecture**. Without virtual memory you cannot
build the phase's capstone (A 5-stage pipelined RISC-V CPU in HDL with assembler.). If two processes each think memory starts at address 0, how does the CPU keep them from clobbering each other?

## The Concept

### Virtual Memory

Every process gets its own **virtual address space**. The OS keeps a **page table** per process mapping virtual pages to physical frames. On context switch, the page table base register is swapped (CR3 on x86).

### Page Table

The page table maps a **virtual page number (VPN)** to a **physical frame number (PFN)**. Typical page size is **4 KB**.

```
Virtual address (32-bit)
┌──────────────────────┬─────────────────┐
│   VPN  (20 bits)     │ Offset (12 bits)│
└──────────────────────┴─────────────────┘
        │                     │
        ▼  page table         ▼  copied directly
Physical address (32-bit)
┌──────────────────────┬─────────────────┐
│   PFN  (20 bits)     │ Offset (12 bits)│
└──────────────────────┴─────────────────┘
```

**Formulas:** `VPN = virtual_address >> 12`, `Offset = virtual_address & 0xFFF`, `PA = (PFN << 12) | Offset`.

**Worked example:** VA `0x00403A7C`. VPN = `0x00403`, Offset = `0xA7C`. Page table maps VPN 0x00403 → PFN 0x007B1. PA = `0x007B1A7C`.

A flat table for 32-bit / 4 KB pages needs 2^20 × 4 bytes = **4 MB** — wasted if the address space is sparse.

### Multi-Level Page Table

A **2-level** table splits the VPN into two 10-bit indices:

```
┌────────────┬────────────┬──────────┐
│ Dir (10b)  │ Table (10b)│Off (12b) │
└────────────┴────────────┴──────────┘
      │              │
      ▼              ▼
 Page Dir ──────> Page Table ──────> Frame
 (1024 entries)   (1024 entries)
```

Only allocated directory entries get page tables. A 4 MB process needs 1 dir + 1 table = **8 KB** instead of 4 MB. On **64-bit x86-64** (4-level): `PML4 → PDP → PD → PT → frame`, each index 9 bits.

### TLB — Translation Lookaside Buffer

The TLB is a small on-chip cache of recent VPN→PFN translations.

| | Cost |
|---|---|
| TLB hit | ~1 cycle |
| TLB miss → page table walk | 100–1000 cycles |
| Page fault → disk load | ~10,000,000 cycles (~10 ms) |

Typical TLB: 64–1536 entries. On context switch the TLB is flushed or tagged with an **ASID**. Associativities: direct-mapped, set-associative (2/4/8-way — common hardware choice), fully-associative.

### Page Fault

When the PTE is **invalid**, hardware raises a page fault exception. The OS: finds/evicts a frame (LRU etc.), writes victim back if dirty, loads the requested page, updates the PTE, restarts the instruction. Page faults are **extremely expensive** — a thrashing program crawls.

### MMU — Memory Management Unit

The MMU sits between the CPU and memory bus. On every load/store it extracts VPN+offset, checks the TLB, walks the page table on miss, and raises an exception on fault. It also enforces permissions (R/W/X) and tracks dirty/referenced bits.

## Build It

We build a VM simulator modeling a 2-level page table, configurable TLB, and LRU replacement. See `code/main.c`.

### Step 1: Data Structures

```c
typedef struct {
    int valid, dirty, referenced;
    int frame_number;   // -1 if invalid
} PageTableEntry;

typedef struct {
    PageTableEntry *directory[1024]; // NULL = not allocated
} PageTable;
```

### Step 2: TLB

```c
typedef struct {
    int valid;
    unsigned int vpn;
    int pfn;
    int last_used;      // LRU timestamp
} TLBEntry;

typedef struct {
    TLBEntry entries[64];
    int associativity;   // 1 = direct-mapped, N = N-way
    int time_counter, hits, misses;
} TLB;
```

### Step 3: Translation

`mmu_translate(tlb, pt, vpn)`: TLB lookup → page table walk on miss → page fault handler on invalid. Hit returns PFN immediately; miss inserts into TLB; fault allocates a frame via LRU replacement.

### Step 4–5: LRU Replacement & Simulation

Evict the frame with the oldest `last_used` timestamp. Feed sequential/random/working-set traces and collect stats: total accesses, TLB hits/misses, page faults.

## Use It

Every modern OS uses virtual memory. **Linux**: `arch/x86/mm/fault.c` (fault handler), `arch/x86/include/asm/pgtable.h` (PTE bits). **Windows**: `MmTranslateVirtualAddress`. **macOS/XNU**: `pmap` module. Real OSes add huge pages, COW, mmap, and swap — our simulator covers the core lookup path.

## Read the Source

- `arch/x86/mm/fault.c` — Linux page fault handler.
- `arch/x86/include/asm/pgtable_types.h` — PTE bit definitions.
- Intel SDM Vol. 3A, Ch. 4 — x86 paging hardware spec.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`vm_sim` — a self-contained virtual memory simulator you can reuse in later phases.**

## Exercises

1. **Easy** — Implement the 2-level page table lookup from scratch. Verify `0x00403A7C` → `0x007B1A7C`.
2. **Medium** — Simulate TLB with three associativities (direct-mapped, 4-way, fully-assoc). Compare hit rates and explain why set-associative is the hardware compromise.
3. **Hard** — Implement LRU and clock (second-chance) replacement. Run on a 2000-access random trace with 32 frames. Compare fault counts.

## Key Terms

| Term | What it means |
|------|---------------|
| Virtual address space | Per-process virtual→physical mapping enforced by the MMU at every load/store. |
| Page / Frame | 4 KB unit of virtual memory (page) or physical DRAM (frame). |
| TLB | On-chip cache (64–1536 entries) of VPN→PFN translations. Hit = 1 cycle. |
| Page fault | Exception when PTE is invalid; triggers ~10 ms disk load. |
| MMU | Hardware unit translating virtual→physical and enforcing permissions. |
| Page table walk | Traversing page directory + tables in DRAM — 100–1000 cycles. |
| Working set | Pages a process actively touches; if it fits in RAM, no thrashing. |

## Further Reading

- *Computer Architecture: A Quantitative Approach* (Hennessy & Patterson), Ch. 2.
- *Operating Systems: Three Easy Pieces* (Arpaci-Dusseau), Ch. 18–22.
- Intel SDM Vol. 3A, Ch. 4 — Paging.
