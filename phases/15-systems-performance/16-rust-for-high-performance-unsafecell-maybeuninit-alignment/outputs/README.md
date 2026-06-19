# Rust Performance Reference Card — Unsafe Patterns, Alignment, Benchmarking

## Interior Mutability Hierarchy

| Type | Thread Safety | Overhead | Use When |
|------|---------------|----------|----------|
| `Cell<T>` | Single-threaded | 1 atomic swap (Copy types) | Simple Copy values, no borrow tracking needed |
| `RefCell<T>` | Single-threaded | Branch + counter per borrow | Non-Copy types, need runtime borrow checking |
| `UnsafeCell<T>` | Single-threaded | 0 (raw ptr deref) | Hot paths where you can prove single-borrow externally |
| `AtomicCell<T>` | Multi-threaded | 1 atomic CAS | Lock-free cross-thread access (crossbeam) |
| `Mutex<T>` | Multi-threaded | Syscall on contention | Multi-threaded, blocking is acceptable |
| `RwLock<T>` | Multi-threaded | Syscall + reader count | Read-heavy multi-threaded workloads |

### UnsafeCell Pattern

```rust
use std::cell::UnsafeCell;

struct FastCounter {
    value: UnsafeCell<u64>,
}

impl FastCounter {
    fn increment(&self) {
        // SAFETY: Single-threaded; no concurrent &mut or & references exist.
        unsafe { *self.value.get() += 1 }
    }
}
```

**Safety invariant:** You must ensure no `&T` and `&mut T` references to the interior exist simultaneously. `UnsafeCell` only opts out of the `&T`→immutable guarantee; it does NOT allow creating overlapping mutable references.

---

## MaybeUninit — Avoid Zero-Initialization

### Pattern: Uninitialized Buffer

```rust
use std::mem::MaybeUninit;

// Allocate without memset
let mut buf: Vec<MaybeUninit<u64>> = Vec::with_capacity(1024);
unsafe { buf.set_len(1024); }
for i in 0..1024 {
    buf[i].write(i as u64);
}

// Convert to initialized — YOU guarantee all elements are written
let buf: Vec<u64> = unsafe {
    let ptr = buf.as_mut_ptr() as *mut u64;
    let len = buf.len();
    std::mem::forget(buf);
    Vec::from_raw_parts(ptr, len, len)
};
```

### Pattern: Array with MaybeUninit

```rust
let mut arr: [MaybeUninit<u64>; 1024] = MaybeUninit::uninit_array();
for i in 0..1024 {
    arr[i].write(i as u64);
}
let arr: [u64; 1024] = arr.map(|slot| unsafe { slot.assume_init() });
```

### Rules

1. **Never** call `.assume_init()` on a `MaybeUninit` that hasn't been `.write()`'n — instant UB.
2. **Never** create a `&T` or `&mut T` to uninitialized memory — that's also UB.
3. Use `.write()` to initialize, `.assume_init_read()` or `.assume_init()` to consume.

---

## ManuallyDrop — Skip Auto-Destruction

```rust
use std::mem::ManuallyDrop;

let v = ManuallyDrop::new(vec![1, 2, 3]);
// v will NOT be dropped at end of scope.

// Explicit drop when YOU decide:
unsafe { ManuallyDrop::drop(&mut v); }
```

**Use cases:** Custom allocators, FFI ownership transfers, extracting inner values without drop.

**Pitfall:** Forgetting to `ManuallyDrop::drop` leaks memory.

---

## Alignment Guide

### repr Attributes

| Attribute | Effect | Use When |
|-----------|--------|----------|
| `#[repr(C)]` | C-compatible layout, predictable padding | FFI, type punning |
| `#[repr(C, packed)]` | No padding between fields | Wire formats, FFI headers only; **never hot paths** |
| `#[repr(align(N))]` | Force alignment to N bytes | False-sharing avoidance (N=64 for cache lines) |
| `#[repr(transparent)]` | Same layout as inner type | Newtype wrappers |
| `#[repr(simd)]` | SIMD vector register layout | Data-parallel operations (use `wide` crate on stable) |

### Layout Examples

```
#[repr(C)]                     #[repr(C, packed)]
struct NormalC {                struct PackedC {
    a: u8,   // offset 0           a: u8,   // offset 0
    b: u64,  // offset 8           b: u64,  // offset 1 (!)
    c: u32,  // offset 16          c: u32,  // offset 9
}                              }
// size=24, align=8             // size=13, align=1
```

### Accessing Packed Fields Safely

```rust
// WRONG — creates misaligned reference, which is UB:
// let x = &packed_struct.b;

// CORRECT — use unaligned read/write:
let val = unsafe { ptr::read_unaligned(&packed_struct.b) };
unsafe { ptr::write_unaligned(&mut packed_struct.b, val + 1) };
```

### Cache-Line Alignment

```rust
#[repr(align(64))]
struct CacheLinePadded<T> {
    value: T,
}
// Each instance starts on a 64-byte boundary.
// Prevents false sharing between threads.
```

---

## Benchmarking Tips

### black_box — Prevent Optimizer Cheating

```rust
use std::hint::black_box;

fn bench() {
    let start = Instant::now();
    let result = compute(black_box(input1), black_box(input2));
    black_box(result); // prevent dead-code elimination
    start.elapsed()
}
```

**Rules:**
1. Wrap *inputs* in `black_box()` — prevents constant-folding.
2. Wrap *outputs* in `black_box()` — prevents dead-code elimination.
3. Use `criterion` for statistical rigor, not `Instant::now()`.

### Criterion Setup

```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "my_bench"
harness = false
```

---

## transmute — When It's Justified

**Justified uses:**
- FFI interop with `#[repr(C)]` types of the same size
- Zero-copy parsing of byte slices into structured data (with alignment checks)
- Converting between types that share identical `#[repr(C)]` layout

**Justified alternatives:**
- `.to_bits()` / `.from_bits()` for float↔int conversion
- `.cast()` for pointer type changes
- `bytemuck` crate for safe casting of `#[repr(C)]` types
- `zerocopy` crate for safe byte parsing

**Always assert size equality:**

```rust
assert_eq!(std::mem::size_of::<Src>(), std::mem::size_of::<Dst>());
```

---

## Pin — Self-Referential Safety

```rust
struct SelfReferential {
    data: String,
    pointer: *const u8, // points into self.data
}

impl SelfReferential {
    fn new(s: &str) -> Pin<Box<Self>> {
        let data = s.to_string();
        let pointer = data.as_ptr();
        Pin::new(Box::new(SelfReferential { data, pointer }))
    }
}
```

**Key insight:** `Pin` prevents *move*, not *mutation*. You can still get `&mut T` via `DerefMut`.

**When you need Pin:**
- `async` blocks and futures (they store references across yield points)
- Self-referential structs where a pointer refers to data within the same allocation

---

## Unsafe Decision Framework

```
1. PROFILE FIRST — Does the safe version appear in your flame graph?
   NO  → Stop. Don't optimize what isn't slow.
   YES → Continue.

2. BENCHMARK — Does the unsafe version measure faster with black_box?
   NO  → Don't use it. You added risk without gain.
   YES → Continue.

3. MINIMIZE — Is the unsafe block ≤ 10 lines with a clear SAFETY comment?
   NO  → Refactor into smaller scoped blocks with invariants documented.
   YES → Continue.

4. VERIFY — Run `cargo +nightly miri test` to catch UB.
   FAILS → Fix the UB before shipping.
   PASSES → Ship with confidence.
```

---

## Common Pitfalls Checklist

- [ ] **Uninitialized reads:** Never `.assume_init()` without `.write()`.
- [ ] **Aliasing violations:** `UnsafeCell` allows mutation through `*mut`, not overlapping `&mut`.
- [ ] **Forgetting ManuallyDrop::drop:** Memory leak.
- [ ] **Packed field references:** `&packed.field` is UB. Use `ptr::read_unaligned`.
- [ ] **transmute size mismatch:** Always assert `size_of::<Src>() == size_of::<Dst>()`.
- [ ] **Unsafe without measurement:** If the benchmark doesn't show improvement, don't use it.
- [ ] **Assuming Pin prevents mutation:** Pin prevents *move*, not *mutation*.