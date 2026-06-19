# Structs, Unions, Bitfields, Alignment

> Struct layout = field order + alignment + padding. Three rules. The reason `sizeof(struct {char a; int b;})` is 8 and not 5.

**Type:** Build
**Languages:** C
**Prerequisites:** Phase 02, Lessons 02, 05
**Time:** ~60 minutes

## Learning Objectives

- Compute the size and layout of a C struct from first principles: alignment, padding, field order.
- Use `offsetof`, `_Alignof`, `_Alignas`, and packed-struct attributes to inspect and override layout.
- Apply unions for type punning safely; recognize bitfields as a compact storage idiom (and their portability hazards).
- Optimize struct layout for cache (place hot fields together, sort fields by descending alignment to minimize padding).

## The Problem

The same struct can be one size on x86, another on ARM, another on Windows — *with the same compiler*, even — because of layout rules. Same code, different output.

Three concrete bugs this lesson prevents:

1. "I sent this struct over the network and the receiver parsed it wrong." (Padding differs by ABI.)
2. "My SIMD load segfaults randomly." (Mis-aligned address; SIMD requires 16/32/64-byte alignment.)
3. "The compiler turned my packed bitfield into 1024 bytes." (Bitfield ABI is implementation-defined; some compilers expand them.)

## The Concept

### Alignment

Most architectures require multi-byte loads to occur at addresses divisible by the load size:

- `int` (4 bytes) at address divisible by 4.
- `double` (8 bytes) at address divisible by 8.
- 16-byte SIMD load at address divisible by 16.

Violating alignment may be slow (extra cycles on x86) or trap (SIGBUS on ARM, RISC-V). The compiler arranges variables to satisfy alignment automatically — at the cost of *padding bytes*.

### Struct layout rules (System V / Itanium ABI)

1. Lay out fields in declaration order.
2. Each field is placed at the next address satisfying its **alignment requirement** (the alignment of its type).
3. The struct's overall **size** is rounded up to be a multiple of its **largest member's alignment**.

Example:

```c
struct A {
    char  a;      /* 1 byte */
    int   b;      /* 4 bytes, aligned to 4 */
    char  c;      /* 1 byte */
};
```

Layout:

```
  byte:  0    1 2 3    4    5 6 7    8
        [a] [pad pad pad] [b b b b] [c] [pad pad pad]
  size = 12 (rounded up to multiple of 4 = alignof(int))
```

So `sizeof(struct A) == 12`, not 6. Three bytes of padding after `a` to align `b`; three bytes after `c` to make the struct's size a multiple of 4.

### Field-order optimization

Reorder fields by *descending alignment* (or size, for primitives) to minimize padding:

```c
struct B {       /* better layout */
    int   b;     /* 4 bytes, aligned to 4 */
    char  a;
    char  c;
    /* 2 bytes trailing padding */
};
/* sizeof(B) == 8 — 4 bytes saved vs A */
```

For a large struct or many of them in memory, packing can save substantial cache footprint.

### `_Alignof`, `_Alignas`, `offsetof`

| Macro / keyword | What it tells you / does |
|-----------------|--------------------------|
| `_Alignof(T)` (C11) | Alignment requirement of type T |
| `_Alignas(N)` (C11) | Force a field's alignment to N |
| `offsetof(struct T, member)` (`<stddef.h>`) | Byte offset of member within T |
| `__attribute__((packed))` | Force zero padding (compilers: gcc/clang) |
| `__attribute__((aligned(N)))` | Force alignment of a variable/type to N |

### Unions

A `union` is a single storage location that can be interpreted as any of its declared types:

```c
union U {
    int    i;
    float  f;
};
```

`sizeof(union U) == max(sizeof(int), sizeof(float)) = 4`. Reading a field through a different one than the last write is technically UB (strict aliasing), *except* unions are an explicit exception in the C standard — type-punning through a union is *defined* in C99+ (GNU C and most modern compilers).

Useful for: tagged unions (sum types), bit-level reinterpretation of floats, network-protocol header parsing.

### Bitfields

You can pack multiple sub-byte values into one storage word:

```c
struct flags {
    unsigned int active   : 1;
    unsigned int priority : 3;
    unsigned int channel  : 4;
};
/* sizeof(struct flags) is typically 4 — one int. */
```

Bitfields are convenient but their layout (bit order, packing across word boundaries) is **implementation-defined**. Never assume the wire format matches your local layout. For serialization, manually shift and mask bits.

### Packed structs

`__attribute__((packed))` tells the compiler to remove all padding:

```c
struct __attribute__((packed)) P {
    char a;
    int  b;
    char c;
};
/* sizeof(P) == 6 — no padding */
```

Cost: every access to `b` is potentially mis-aligned. On x86 this is slow; on ARM/RISC-V it may trap. Use for network protocols (where the wire format is byte-exact and the CPU is x86) but never for hot in-memory data.

### Cache-line considerations

A typical cache line is 64 bytes. Two ramifications:

- **False sharing**: two threads writing to fields in the same cache line cause cache invalidations (Phase 13). Pad to 64 bytes to isolate.
- **Hot/cold split**: place fields accessed together adjacent so a single cache miss brings them in.

## Build It

Open `code/main.c`.

### Step 1: Print struct sizes and offsets

`struct A {char a; int b; char c;}` — expected size 12, offsets 0, 4, 8.

### Step 2: Reorder for compactness

`struct B {int b; char a; char c;}` — size drops to 8.

### Step 3: Packed version

`__attribute__((packed))` removes padding — size = 6. Cost: misaligned `b`.

### Step 4: Union type-punning

Float ↔ uint32_t through a union — defined behavior (C99+).

### Step 5: Bitfield

A flags struct packed into one int.

## Use It

- **Network protocols** (Phase 09): parsing IP/TCP headers requires knowing exact byte offsets; use packed structs or manual shift/mask.
- **Filesystems / databases** (Phase 10): on-disk layouts care about exact size and alignment; padding wastes disk.
- **OS / kernel** (Phase 07): system-call structures are ABI-defined and must match the kernel's expectation.
- **SIMD** (Phase 13/15): vectorized loads require alignment; `_Alignas(32)` forces it.
- **Cache optimization** (Phase 15): the difference between a 64-byte and a 96-byte struct can double cache misses on iteration.

## Read the Source

- *The C Programming Language* (K&R), §6 — structs and unions.
- [Eric Raymond's "The Lost Art of Structure Packing"](http://www.catb.org/esr/structure-packing/) — practical guide to manual packing.
- *Linux Device Drivers* (Corbet et al.), Chapter 11 — alignment in kernel-userspace boundaries.

## Ship It

This lesson ships **`outputs/struct_layout.c`** — a utility that, given a struct, prints field offsets and the padding bytes between them. Generalizes to arbitrary structs via macros.

## Exercises

1. **Easy.** Manually predict the size of `struct { char a; double b; char c; }`. Verify with a printf.
2. **Medium.** Reorder the fields of a 5-member struct to minimize sizeof. Confirm with `offsetof` for each member.
3. **Hard.** Implement a TCP-header parser using only `memcpy` and shift/mask (no packed structs). The function should be portable across endianness.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Alignment | "Address divisibility" | The requirement that an N-byte load occur at an address divisible by N (or by some N' ≥ N) |
| Padding | "Wasted bytes" | Bytes inserted between fields to satisfy alignment |
| Packed | "No padding" | `__attribute__((packed))` removes padding at the cost of unaligned access |
| Bitfield | "Bit-packed field" | A struct member specifying its width in bits; layout is implementation-defined |
| `offsetof` | "Field byte offset" | A macro that returns the byte offset of a struct member, used for serialization |

## Further Reading

- [The System V AMD64 ABI](https://gitlab.com/x86-psABIs/x86-64-ABI), Section 3.1 — struct passing rules.
- *Hacker's Delight* by Henry Warren — bit-manipulation reference; useful with bitfields.
- [The Linux kernel coding style on structs](https://www.kernel.org/doc/html/latest/process/coding-style.html) — practical conventions.
