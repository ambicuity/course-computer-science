# Lesson 08: Virtual Memory in the OS — Demand Paging

## Core Concepts

Every process sees its own private **virtual address space**. The OS maps virtual addresses to physical addresses via **page tables**. This indirection enables isolation, sharing, and the illusion of abundant memory.

## Virtual Address Space Layout

A typical 64-bit process address space:

```
0x0000_0000_0000_0000  ┌─────────────────┐
                       │  Code (text)     │  ← read-execute
                       ├─────────────────┤
                       │  Data            │  ← initialized globals
                       ├─────────────────┤
                       │  BSS             │  ← zero-initialized
                       ├─────────────────┤
                       │  Heap  ──►       │  ← grows up (malloc)
                       │                  │
                       │       ◄── Heap   │
                       ├─────────────────┤
                       │  (unmapped gap)  │
                       ├─────────────────┤
                       │       ◄── Stack  │  ← grows down
                       │  Stack           │
0xFFFF_FFFF_FFFF_FFFF  └─────────────────┘
```

Each region is a set of **virtual pages** (typically 4 KB). The page table maps each valid virtual page number (VPN) to a physical frame number (PFN).

## Page Tables

The page table is a per-process data structure. Each **page table entry (PTE)** contains:

| Field | Purpose |
|-------|---------|
| Valid bit | Is this page mapped and in memory? |
| PFN | Physical frame number (if valid) |
| Protection | Read / Write / Execute permissions |
| Dirty bit | Has this page been written to? |
| Reference bit | Has this page been accessed recently? |

Multi-level page tables (e.g., 4-level on x86-64) avoid allocating entries for sparse address spaces. The VPN is split into indices for each level:

```
VPN = [PML4 | PDPT | PD | PT]  →  offset within page
```

## Demand Paging

Instead of loading the entire program into memory at startup, the OS loads pages **on demand** (lazy allocation). Pages that are never accessed are never loaded.

**Page fault handler sequence:**
1. Process accesses a virtual address
2. MMU walks the page table, finds PTE with valid bit = 0
3. MMU raises a **page fault** trap to the OS
4. OS checks: is this a valid address? (segfault if not)
5. OS finds a free physical frame (or evicts one — Lesson 09)
6. If backed by a file: load from disk. If anonymous: zero-fill
7. Update the PTE: set valid bit, store PFN
8. Return from trap; the faulting instruction restarts

```
Process: MOV RAX, [0x1234]
                    │
         ┌──────────▼──────────┐
         │ MMU: page table walk │
         │ valid bit = 0        │
         └──────────┬──────────┘
                    │ PAGE FAULT (trap)
         ┌──────────▼──────────┐
         │ OS fault handler     │
         │ allocate frame #42   │
         │ zero-fill            │
         │ PTE: valid=1, PFN=42 │
         └──────────┬──────────┘
                    │ return
Process: MOV RAX, [0x1234]  ← retry, now succeeds
```

The first access to a page is expensive (disk I/O ~5ms for HDD, ~0.1ms for SSD). Subsequent accesses are fast (memory speed).

## Copy-on-Write (COW)

When `fork()` creates a child process, duplicating all pages is wasteful (most pages will be overwritten or never modified). Instead:

1. Parent and child share the same physical pages
2. Both page tables point to the same PFNs
3. All shared pages are marked **read-only**
4. If either process writes to a page, a page fault occurs
5. The OS copies the page, updates the faulting process's PTE, marks it writable
6. Both processes now have independent copies

```
Before fork():    [Page A] → physical frame 100

After fork():     Parent PTE → frame 100 (read-only)
                  Child  PTE → frame 100 (read-only)

After child writes:
                  Parent PTE → frame 100 (read-write)
                  Child  PTE → frame 200 (read-write)  ← copied
```

COW makes `fork()` nearly instantaneous regardless of process size.

## Memory-Mapped Files (`mmap`)

`mmap` maps a file (or anonymous region) into the virtual address space. The file's contents appear as an array in memory.

```c
void *p = mmap(NULL, length, PROT_READ | PROT_WRITE,
               MAP_SHARED, fd, 0);
```

**Benefits:**
- File I/O via memory loads/stores instead of `read()`/`write()`
- Page faults load file contents on demand
- Shared mappings allow IPC between processes
- The OS can flush dirty pages back to the file

## Swap

When physical memory is full, the OS evicts pages to a **swap partition** (or swap file) on disk. The PTE is updated: valid bit cleared, and the PFN field stores the swap offset. When the page is accessed again, a page fault triggers swapping it back in.

The balance between working set size and physical memory determines **thrashing** — when the system spends more time swapping than executing useful work.

## Build It

Write a demand paging simulator in C. Simulate a virtual address space with a page table. Implement the page fault handler, copy-on-write simulation, and a simplified `mmap`. Track page faults, frame usage, and swap operations.

## Use It

Every modern OS uses demand paging: Linux, macOS, Windows, FreeBSD, Android, iOS. The `fork()` + `exec()` pattern relies on COW. Databases use `mmap` for efficient file access. Containers benefit from shared page tables via COW.

## Ship It

Your paging simulator should process a sequence of virtual addresses, trigger page faults on first access, handle COW on writes to shared pages, and report statistics (faults, frames used, swap-ins/outs).

## Exercises

### Level 1 — Concept Check
A process has a 32-bit virtual address space with 4 KB pages. How many virtual pages exist? If each PTE is 8 bytes, how much memory does a single-level page table consume?

### Level 2 — Implementation
Extend the simulator to support multi-level page tables (2 levels). Only allocate second-level tables when needed. Compare memory overhead vs. a flat table for a sparse address space.

### Level 3 — Design
Design a demand-paging system that handles memory-mapped files with COW. Describe the PTE format, the fault handler logic, and how dirty pages are written back to the file. Implement the fault handler.
