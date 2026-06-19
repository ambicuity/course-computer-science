# Rust for High Performance — UnsafeCell, MaybeUninit, alignment

> Rust's safety guarantees are zero-cost *until they aren't*. This lesson shows you exactly where the compiler inserts hidden costs, and how to opt out responsibly when every nanosecond matters.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 15 lessons 01–15
**Time:** ~75 minutes

## Learning Objectives

- Understand the safety/performance tradeoff in Rust and where the compiler adds hidden runtime costs.
- Use `UnsafeCell`, `Cell`, `RefCell`, and `AtomicCell` for interior mutability without unnecessary overhead.
- Use `MaybeUninit` to avoid zero-initialization costs for arrays and buffers.
- Use `ManuallyDrop` to take manual control of destruction semantics.
- Control memory alignment with `repr(align(N))`, `repr(C)`, `repr(C, packed)`, and understand the performance impact of misalignment.
- Understand `repr(simd)` and portable SIMD for data-parallel operations.
- Know when `std::mem::transmute` is justified and when it's a footgun.
- Use `std::hint::black_box` for honest benchmarking.
- Understand `Pin` and why self-referential structs need it.
- Know when `unsafe` is actually worth it and when safe alternatives are already optimal.

## The Problem

You're building a low-latency trading engine. Your hot path processes 10 million events per second. You profile and discover:

1. **Hidden allocation from `RefCell` runtime checks** — every borrow costs a branch and a counter update.
2. **Zero-initialization of a 64 KB buffer** — `let buf = [0u8; 65536]` memsets 64 KB on every call, even though you overwrite every byte moments later.
3. **Misaligned access to a `#[repr(C)]` struct** — the compiler padded your struct to 128 bytes when 64 suffices, halving your cache-line density.
4. **The optimizer ate your benchmark** — `std::hint::black_box` is missing from your microbenchmarks, so the compiler elided the entire computation.

Each of these costs is *invisible* in safe Rust. The language promises zero-cost abstractions, but only for abstractions whose cost you can see in the source. When you need to go faster, you must understand what the compiler is doing behind the curtain.

## The Concept

### Rust's Safety/Performance Tradeoff

Rust's borrow checker is a compile-time analysis. At runtime, the generated code should be identical to hand-written C — that's the "zero-cost abstraction" promise. But there are places where Rust inserts *runtime* safety mechanisms:

| Mechanism | Runtime Cost | Avoidable? |
|-----------|-------------|------------|
| `RefCell` borrow counter | Branch + atomic counter per borrow | Yes — use `UnsafeCell` + manual discipline |
| Bounds checking on indexing | Branch per access | Yes — use `.get_unchecked()` in hot loops |
| Zero-init of `mem::zeroed()` | `memset` per allocation | Yes — use `MaybeUninit` |
| `Drop` glue | Function call per scope exit | Maybe — use `ManuallyDrop` |
| Integer overflow checks | Branch per arithmetic op | Yes — use `wrapping_*` / `unchecked_*` |

The `unsafe` keyword is your escape hatch. It doesn't turn off the borrow checker — it tells the compiler *you* accept responsibility for an invariant it can't verify. The key rule: **use `unsafe` blocks that are small, documented, and auditable.**

### UnsafeCell and Interior Mutability

`UnsafeCell<T>` is the *only* legal way to mutate through a `&T` in Rust. It's the primitive that `Cell`, `RefCell`, `AtomicCell`, and `Mutex` are built on.

```rust
use std::cell::UnsafeCell;

struct SharedCounter {
    value: UnsafeCell<u64>,
}

// UnsafeCell opts out of the aliasing guarantee:
// &SharedCounter can now be used to mutate value.
// YOU must guarantee no data races exist.
```

**Why not just use `RefCell`?** `RefCell` adds:

- A `Cell<usize>` borrow state (8 bytes overhead)
- A runtime check on every `borrow()` / `borrow_mut()`
- A `panic!` on double-borrow instead of UB

In single-threaded hot paths where you *know* only one borrow exists at a time, `UnsafeCell` skips all of that. The cost difference:

| Type | Overhead per access | Memory overhead |
|------|---------------------|-----------------|
| `RefCell<T>` | 1 branch + 2 atomic ops | `size_of::<T>() + size_of::<usize>()` |
| `UnsafeCell<T>` | 0 (raw ptr deref) | `size_of::<T>()` |
| `Cell<T>` (copy types) | 1 atomic swap | `size_of::<T>()` |
| `AtomicCell<T>` (crossbeam) | 1 atomic CAS | `size_of::<T>()` |

**When to use what:**
- `Cell<T>` — For `Copy` types in single-threaded contexts. Cheap, safe, zero-heap.
- `RefCell<T>` — For non-`Copy` types in single-threaded contexts. Has runtime borrow checking.
- `UnsafeCell<T>` — When you can prove single-borrow invariants externally and need every nanosecond.
- `AtomicCell<T>` — For lock-free cross-thread access (crossbeam crate).

### MaybeUninit for Uninitialized Memory

`MaybeUninit<T>` tells the compiler "this memory may not be valid yet." This matters because:

1. **Rust forbids uninitialized reads** — reading `mem::uninitialized()` is instant UB.
2. **Zero-init costs** — `let buf = vec![0u64; N]` calls `memset` even if you write every element before reading.
3. **The compiler can optimize around assumed-validity** — if the compiler *thinks* a value is valid, it may optimize based on that assumption, leading to miscompilation.

```rust
use std::mem::MaybeUninit;

// Allocate without initialization — no memset
let mut buf: [MaybeUninit<u64>; 1024] = MaybeUninit::uninit_array();

// Fill it
for i in 0..1024 {
    buf[i].write(i as u64);
}

// Convert to initialized array — YOU guarantee all elements are written
let buf: [u64; 1024] = buf.map(|slot| unsafe { slot.assume_init() });
```

**The rule:** Use `MaybeUninit` when you have a large buffer/array that you will fully initialize before reading *every* element, and the zero-init cost matters.

**Cost comparison for a 64 KB buffer (1024 × `u64`):**

| Method | Time (approx.) | memset call? |
|--------|----------------|--------------|
| `vec![0u64; 1024]` | ~8 µs | Yes |
| `MaybeUninit::uninit_array()` | ~0 ns | No |
| `Box::new_uninit_slice(1024)` | ~0 ns | No |

On a hot path that allocates per event, eliminating the `memset` can save 1–5% of total CPU.

### ManuallyDrop

`ManuallyDrop<T>` wraps a value and prevents the compiler from inserting `Drop` glue. This is useful when:

- You're implementing a custom allocator and need to free memory *after* some other operation.
- You're moving a value out of a container without triggering its destructor (e.g., `Vec` swapping).
- You're implementing a self-referential or FFI type where drop order matters critically.

```rust
use std::mem::ManuallyDrop;

let mut v = ManuallyDrop::new(vec![1, 2, 3]);
// v will NOT be dropped when it goes out of scope.

// If you need to drop it manually:
unsafe { ManuallyDrop::drop(&mut v); }
```

**Warning:** Forgetting to `ManuallyDrop::drop` leaks memory. Forgetting to wrap in `ManuallyDrop` when you wanted it causes use-after-free. Both are easy mistakes.

### Alignment and `repr` Attributes

Memory alignment determines how data maps to cache lines and hardware boundaries. Misaligned access can:

- Cause a CPU fault on some architectures (ARM, MIPS).
- Force the CPU to do two loads instead of one (x86 penalty: 2–5× slower).
- Reduce cache-line density (fewer elements per 64-byte line).

#### `repr(align(N))`

Forces a type to have alignment `N`, padding it out if needed.

```rust
#[repr(align(64))]
struct CacheLineAligned {
    data: [u64; 8], // exactly 64 bytes, now cache-line aligned
}
```

**When to use:** False-sharing avoidance in concurrent data structures. Each `CacheLineAligned` lands on its own cache line, so separate threads don't invalidate each other's caches.

#### `repr(C)` vs `repr(C, packed)`

```rust
#[repr(C)]
struct NormalC {
    a: u8,   // offset 0
    b: u64,  // offset 8 (6 bytes padding)
    c: u32,  // offset 16
} // size = 24 (padded to align 8)

#[repr(C, packed)]
struct PackedC {
    a: u8,   // offset 0
    b: u64,  // offset 1 (!)
    c: u32,  // offset 9
} // size = 13 (no padding)
```

Accessing `PackedC::b` on x86 requires the compiler to emit byte-by-byte loads because it's misaligned. On ARM, it's a bus error. **Use `packed` only for wire formats and FFI headers.** Never for hot-path data structures.

**Alignment rule of thumb:** Match your type's alignment to its largest field, or to the cache line size (64 bytes) when you need to avoid false sharing.

#### `repr(simd)` and Portable SIMD

`repr(simd)` makes a struct layout-compatible with a SIMD vector register. `std::simd` (Nightly) and the `wide` crate provide portable SIMD abstractions:

```rust
#[repr(simd)]
struct f32x4([f32; 4]); // maps to __m128 on x86, float32x4_t on ARM

// Portable SIMD (Nightly):
use std::simd::*;
let a = f32x4::splat(1.0);
let b = f32x4::splat(2.0);
let c = a + b; // one SIMD instruction
```

**When to use:** Data-parallel operations over 4–32 `f32`/`f64`/`i32` values. Cryptography, image processing, game math, ML inference. The `wide` crate gives you this on stable Rust.

### `std::mem::transmute` and When You Need It

`transmute` reinterprets the bit pattern of a value as a different type. It is the nuclear option:

```rust
let bits: u32 = 0x41424344;
let chars: [u8; 4] = unsafe { std::mem::transmute(bits) };
// chars == [0x44, 0x43, 0x42, 0x41] on little-endian
```

**When it's justified:**
- FFI interop where C passes you a type-punned value.
- Implementing a zero-copy parser that reinterprets byte slices as structured data (with alignment checks).
- Converting between `#[repr(C)]` types that share a layout.

**When it's a footgun:**
- Converting between types of different sizes (instant UB).
- Converting `&T` to `&U` where `U` has different validity requirements.
- Any use where `from_bits`, `to_bits`, `cast`, or a safe conversion exists.

**Rule:** If there's a safe alternative, use it. `transmute` should be a last resort, wrapped in a function with a safety comment.

### `std::hint::black_box` for Benchmarking

The optimizer is smart. If you benchmark this:

```rust
fn add(a: u64, b: u64) -> u64 { a + b }

fn bench() {
    let start = Instant::now();
    let result = add(5, 7);
    let elapsed = start.elapsed();
    println!("{:?}", elapsed);
}
```

The compiler will constant-fold `add(5, 7)` into `12` and your benchmark measures nothing. `black_box` prevents this:

```rust
use std::hint::black_box;

fn bench() {
    let start = Instant::now();
    let result = add(black_box(5), black_box(7));
    black_box(&result); // prevent dead-code elimination
    let elapsed = start.elapsed();
    println!("{:?}", elapsed);
}
```

`black_box` is an identity function at runtime but tells the optimizer "assume this value may be used in any way." It's the standard tool for honest microbenchmarks.

### Pin and Self-Referential Structs

`Pin<P>` wraps a pointer `P` and promises the pointed-to value will never be moved. This matters for self-referential types:

```rust
struct SelfReferential {
    data: String,
    pointer: *const u8, // points into `data`
}

// If SelfReferential is moved, pointer becomes dangling!
// Pin<Box<SelfReferential>> guarantees the Box won't be reallocated,
// so pointer stays valid.
```

**Why this matters for performance:**
- `async` blocks are self-referential (they store local references across yield points).
- `Pin` enables `async` without heap allocation in some cases.
- Some data structures (intrusive lists, arena-allocated graphs) need pinning guarantees.

**Pin vs raw pointers:** `Pin` is a type-system guarantee that the value won't be moved. Raw pointers rely on manual discipline. Use `Pin` when the compiler can verify your invariants; use raw pointers when it can't.

### When `unsafe` Is Worth It

| Pattern | Speedup | Risk Level | Recommendation |
|---------|---------|------------|----------------|
| `UnsafeCell` in single-threaded hot path | 5–20% | Low | Worth it with clear safety proof |
| `MaybeUninit` for large buffers | 1–5% | Medium | Worth it if you have clear init invariants |
| `.get_unchecked()` in vectorized loops | 10–50% | Medium | Worth it if bounds are verified once outside |
| `transmute` for type punning | Variable | High | Avoid unless layout is `#[repr(C)]` and documented |
| Raw pointer arithmetic | Variable | High | Minimal `unsafe` blocks, heavy documentation |
| `packed` struct access | Negative–10% | High | Only for FFI; never for hot paths |

**The decision framework:**

1. **Profile first.** If the safe version doesn't show up in your flame graph, stop.
2. **Benchmark before and after.** Use `black_box`. Use `criterion`. Be honest.
3. **Wrap `unsafe` in a safe API.** The `unsafe` block should be 5–10 lines, with a `// SAFETY:` comment.
4. **Test with Miri.** `miri` catches undefined behavior that your tests miss. Run `cargo +nightly miri test`.
5. **Document why the invariant holds.** Future-you will thank present-you.

### Rust vs C++: Zero-Cost Abstractions That Actually Cost Zero

| Abstraction | Rust | C++ | Runtime cost? |
|------------|------|-----|---------------|
| Unique ptr | `Box<T>` | `std::unique_ptr<T>` | Both: 0 cost (no refcount) |
| Shared ptr | `Rc<T>` / `Arc<T>` | `std::shared_ptr<T>` | Both: atomic refcount cost |
| Iterator | `.map().filter()` | ranges/views | Both: 0 cost (inlined) |
| Bounds check | `[i]` (checked) | `.at(i)` (checked) | Both: branch; Rust `unsafe` opt-out |
| Virtual dispatch | `dyn Trait` | `virtual` | Both: vtable indirection |
| Move semantics | Move by default | Move after `std::move` | Both: 0 cost (memcpy) |
| Zero-init | `mem::zeroed()` | `T{}` / `= {}` | Both: memset; Rust has `MaybeUninit` escape |

Rust's true advantage: *the escape hatches are auditable*. Every `unsafe` block is grep-able. Every `unwrap()` is visible. In C++, undefined behavior is silent and ubiquitous.

### Common Pitfalls

1. **Uninitialized reads:** Reading from `MaybeUninit::uninit().assume_init()` is instant UB. Always `.write()` before `.assume_init()`.
2. **Aliasing violations:** `UnsafeCell` doesn't give you a license to create `&mut` while `&` exists. It lets you mutate through `*mut`. Creating overlapping `&mut` references is still UB.
3. **Forgetting to drop `ManuallyDrop`:** Leaking memory is easy. Use `ManuallyDrop::drop` explicitly or wrap in a container that handles it.
4. **Misaligned `packed` access:** Taking a reference to a `packed` field creates a misaligned `&T`, which is UB. Use `ptr::read_unaligned` and `ptr::write_unaligned` instead.
5. **`transmute` size mismatches:** The source and target types must be the same size. The compiler won't always catch this.
6. **Using `unsafe` for speed without measuring:** If your `unsafe` version isn't measurably faster, it's just risk. Always benchmark.
7. **Assuming `Pin` prevents mutation:** `Pin` prevents *move*, not *mutation*. You can still mutate a `Pin<&mut T>` via `DerefMut`.

## Build It

### Step 1: Minimal Version

Create a benchmark comparing `RefCell<u64>` vs `UnsafeCell<u64>` for incrementing a shared counter. Use `black_box` to prevent optimization.

```rust
use std::cell::{Cell, RefCell, UnsafeCell};
use std::hint::black_box;
use std::time::Instant;

fn bench_refcell(n: u64, v: &RefCell<u64>) -> u64 {
    let mut total = 0u64;
    for i in 0..n {
        *v.borrow_mut() += 1;
        total += *v.borrow();
        black_box(&total);
    }
    total
}

fn bench_unsafecell(n: u64, v: &UnsafeCell<u64>) -> u64 {
    let mut total = 0u64;
    for i in 0..n {
        unsafe { *v.get() += 1 }
        total += unsafe { *v.get() };
        black_box(&total);
    }
    total
}
```

### Step 2: Realistic Version

Add `MaybeUninit` buffer comparison, alignment demo, `ManuallyDrop` demo, and `Pin` demo. See `code/main.rs`.

## Use It

In production Rust codebases:

- **`crossbeam`** uses `UnsafeCell` extensively in its epoch-based reclamation (`src/epoch/atomic.rs`). Each slot in the epoch hash is an `UnsafeCell<AtomicPtr>` — the `UnsafeCell` avoids a `RefCell` overhead that would kill throughput at scale.
- **`tikv`** (TiKV's Raft engine) uses `MaybeUninit` for its log buffer to avoid zero-init on the write path (`components/raft_log_engine/src/log_batch.rs`).
- **The Rust standard library** uses `MaybeUninit` in `Vec::with_capacity` — the capacity beyond `len` is `MaybeUninit` to avoid zeroing memory that will be written before reading.
- **`tokio`** uses `Pin` for all async task representations. Every future in Tokio's runtime is `Pin<Box<dyn Future>>`.

Compare your minimal version against `crossbeam::epoch::Owned` — notice how they wrap `UnsafeCell` in a safe abstraction with lifetime tracking, so the `unsafe` block is tiny and audit-able.

## Read the Source

- `crossbeam/src/epoch/atomic.rs` — `UnsafeCell` usage in lock-free epoch-based reclamation
- `rust/library/alloc/src/vec/mod.rs` — `MaybeUninit` usage in `Vec`'s spare capacity
- `tokio/src/runtime/task/mod.rs` — `Pin` usage for async task allocation

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained reference card** (`rust_perf_reference.md`) covering unsafe patterns, alignment rules, and benchmarking tips.

## Exercises

1. **Easy** — Implement a `Cell`-based counter and compare its performance to `RefCell` and `UnsafeCell`. Use `black_box` for honest measurements.
2. **Medium** — Build a `MaybeUninit`-backed ring buffer that avoids zero-initialization on creation. Prove correctness with Miri.
3. **Hard** — Implement a self-referential struct using `Pin<Box<SelfReferential>>` that stores a `String` and a pointer into its data. Prove that moving it would be unsound, and write safe accessor methods.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| UnsafeCell | "Rust's escape hatch for mutation" | The *only* legal way to mutate through `&T`; requires you to guarantee no aliasing violations |
| MaybeUninit | "Uninitialized memory" | A type-level marker that tells the compiler this memory may not be valid; reading it is UB until `.write()` + `.assume_init()` |
| ManuallyDrop | "Skip the destructor" | Wraps `T` so `Drop` glue is never auto-inserted; you must call `ManuallyDrop::drop` yourself or leak |
| repr(align) | "Force alignment" | Sets minimum alignment; pads the type so every instance starts on an `N`-byte boundary |
| Pin | "Can't move this" | A wrapper guaranteeing the value won't be moved; needed for self-referential types and async state machines |
| black_box | "Prevent optimization" | A hint function that tells the optimizer to assume the value may be used in any way; essential for honest benchmarks |
| transmute | "Type punning" | Reinterprets bits as another type; extremely dangerous unless both types share `#[repr(C)]` and same size |

## Further Reading

- [Rust Reference: Undefined Behavior](https://doc.rust-lang.org/reference/behavior-considered-undefined.html) — The definitive list of what's UB in Rust.
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/) — The book on unsafe Rust that this lesson summarizes.
- [Miri](https://github.com/rust-lang/miri) — The tool that catches undefined behavior at runtime. Your best friend.
- [Criterion.rs](https://bheisler.github.io/criterion.rs/book/) — Statistical benchmarking for Rust. Don't benchmark with `Instant::now()` in production; use this.
- [Portable SIMD RFC](https://github.com/rust-lang/rfcs/blob/master/text/3226-portable-simd.md) — The RFC for `std::simd`; still Nightly but the future of SIMD in Rust.