//! Rust for High Performance — UnsafeCell, MaybeUninit, alignment
//! Phase 15 — Systems Programming & Performance
//!
//! Demonstrates unsafe patterns for high-performance Rust:
//!   1. UnsafeCell vs RefCell vs Cell performance
//!   2. MaybeUninit — avoiding zero-init cost
//!   3. ManuallyDrop — manual destruction control
//!   4. Alignment with repr(align) — cache-line alignment
//!   5. Packed structs and their misalignment penalty
//!   6. black_box for honest benchmarking
//!   7. Pin for self-referential safety
//!   8. Safe vs unsafe comparison with timing

use std::cell::{Cell, RefCell, UnsafeCell};
use std::hint::black_box;
use std::mem::{align_of, align_to, MaybeUninit, ManuallyDrop};
use std::pin::Pin;
use std::time::Instant;

// ---------------------------------------------------------------------------
// 1. UnsafeCell vs RefCell vs Cell benchmarking
// ---------------------------------------------------------------------------

fn bench_refcell(n: u64, counter: &RefCell<u64>) -> u64 {
    let mut total = 0u64;
    for _ in 0..n {
        *counter.borrow_mut() += 1;
        total += *counter.borrow();
    }
    black_box(total)
}

fn bench_cell(n: u64, counter: &Cell<u64>) -> u64 {
    let mut total = 0u64;
    for _ in 0..n {
        counter.set(counter.get() + 1);
        total += counter.get();
    }
    black_box(total)
}

// SAFETY: This function is called from a single-threaded context, and we
// guarantee no other references to the UnsafeCell's interior exist
// concurrently. No &mut or & references are created — only raw pointer ops.
fn bench_unsafecell(n: u64, counter: &UnsafeCell<u64>) -> u64 {
    let mut total = 0u64;
    for _ in 0..n {
        // SAFETY: We have exclusive logical access; no aliasing violations.
        unsafe {
            *counter.get() += 1;
            total += *counter.get();
        }
    }
    black_box(total)
}

fn demo_interior_mutability() {
    println!("=== Interior Mutability: RefCell vs Cell vs UnsafeCell ===\n");
    let iterations: u64 = 10_000_000;

    let refcell_counter = RefCell::new(0u64);
    let start = Instant::now();
    let r1 = bench_refcell(iterations, &refcell_counter);
    let refcell_dur = start.elapsed();
    println!("RefCell      : {:?}  (result={})", refcell_dur, r1);

    let cell_counter = Cell::new(0u64);
    let start = Instant::now();
    let r2 = bench_cell(iterations, &cell_counter);
    let cell_dur = start.elapsed();
    println!("Cell         : {:?}  (result={})", cell_dur, r2);

    let unsafe_counter = UnsafeCell::new(0u64);
    let start = Instant::now();
    let r3 = bench_unsafecell(iterations, &unsafe_counter);
    let unsafe_dur = start.elapsed();
    println!("UnsafeCell   : {:?}  (result={})", unsafe_dur, r3);

    println!(
        "\nUnsafeCell is {:.1}x faster than RefCell\n",
        refcell_dur.as_nanos() as f64 / unsafe_dur.as_nanos().max(1) as f64
    );
}

// ---------------------------------------------------------------------------
// 2. MaybeUninit — avoiding zero-init on large buffers
// ---------------------------------------------------------------------------

fn demo_maybeuninit() {
    println!("=== MaybeUninit: Avoiding Zero-Initialization ===\n");

    const BUF_SIZE: usize = 1024 * 64; // 64K u64s = 512 KB

    // Zero-initialized version (memset cost)
    let start = Instant::now();
    let mut zero_buf = vec![0u64; BUF_SIZE];
    for i in 0..BUF_SIZE {
        zero_buf[i] = i as u64;
    }
    black_box(&zero_buf);
    let zero_dur = start.elapsed();
    println!("vec![0; {}]: {:?}", BUF_SIZE, zero_dur);

    // MaybeUninit version (no memset)
    let start = Instant::now();
    let mut uninit_buf: Vec<MaybeUninit<u64>> = Vec::with_capacity(BUF_SIZE);
    unsafe {
        uninit_buf.set_len(BUF_SIZE);
    }
    for i in 0..BUF_SIZE {
        uninit_buf[i].write(i as u64);
    }
    // SAFETY: Every element has been .write()’n above.
    let init_buf: Vec<u64> = unsafe {
        let ptr = uninit_buf.as_mut_ptr() as *mut u64;
        let len = uninit_buf.len();
        std::mem::forget(uninit_buf);
        Vec::from_raw_parts(ptr, len, len)
    };
    black_box(&init_buf);
    let uninit_dur = start.elapsed();
    println!("MaybeUninit  : {:?}", uninit_dur);

    println!(
        "\nMaybeUninit is {:.1}x faster than zero-init\n",
        zero_dur.as_nanos() as f64 / uninit_dur.as_nanos().max(1) as f64
    );
}

// ---------------------------------------------------------------------------
// 3. ManuallyDrop — manual destruction control
// ---------------------------------------------------------------------------

fn demo_manually_drop() {
    println!("=== ManuallyDrop: Manual Destruction Control ===\n");

    let v = vec![1, 2, 3, 4, 5];
    let mut md = ManuallyDrop::new(v);

    // Access the inner value without dropping
    println!("ManuallyDrop value: {:?}", &*md);

    // Extract a reference to inner data before deciding to drop
    let ptr: *const i32 = md.as_ptr();
    println!("Pointer to data: {:?}", ptr);

    // Now drop it explicitly — SAFETY: we haven't moved out of md
    unsafe { ManuallyDrop::drop(&mut md) }
    println!("ManuallyDrop dropped explicitly.\n");
}

// ---------------------------------------------------------------------------
// 4. Alignment with repr(align) — cache-line alignment
// ---------------------------------------------------------------------------

#[repr(align(64))]
struct CacheLineAligned {
    data: [u64; 8],
}

#[repr(C)]
struct NormalStruct {
    a: u8,
    b: u64,
    c: u32,
}

#[repr(C, packed)]
struct PackedStruct {
    a: u8,
    b: u64,
    c: u32,
}

fn demo_alignment() {
    println!("=== Alignment: repr(align), repr(C), repr(C, packed) ===\n");

    println!(
        "NormalStruct      : size={} align={} offset_b={}",
        std::mem::size_of::<NormalStruct>(),
        align_of::<NormalStruct>(),
        offset_of!(NormalStruct, b),
    );
    println!(
        "PackedStruct      : size={} align={} offset_b={}",
        std::mem::size_of::<PackedStruct>(),
        align_of::<PackedStruct>(),
        offset_of!(PackedStruct, b),
    );
    println!(
        "CacheLineAligned  : size={} align={}",
        std::mem::size_of::<CacheLineAligned>(),
        align_of::<CacheLineAligned>(),
    );

    // Benchmark misaligned access
    const N: u64 = 10_000_000;

    let mut packed = PackedStruct { a: 0, b: 0, c: 0 };
    let start = Instant::now();
    for _ in 0..N {
        // SAFETY: Accessing packed field via read/write to avoid UB from &ref
        unsafe {
            let b_ptr = &packed.b as *const u64;
            let val = std::ptr::read_unaligned(b_ptr);
            std::ptr::write_unaligned(&mut packed.b as *mut u64, val + 1);
        }
    }
    let packed_dur = start.elapsed();

    let mut normal = NormalStruct { a: 0, b: 0, c: 0 };
    let start = Instant::now();
    for _ in 0..N {
        normal.b += 1;
        black_box(&normal.b);
    }
    let normal_dur = start.elapsed();

    println!(
        "\nNormal access: {:?}  |  Packed access: {:?}",
        normal_dur, packed_dur
    );
    println!(
        "Misaligned (packed) access is {:.1}x slower\n",
        packed_dur.as_nanos() as f64 / normal_dur.as_nanos().max(1) as f64
    );

    // Cache line aligned array — show addresses
    let arr: [CacheLineAligned; 4] = [
        CacheLineAligned { data: [0; 8] },
        CacheLineAligned { data: [1; 8] },
        CacheLineAligned { data: [2; 8] },
        CacheLineAligned { data: [3; 8] },
    ];
    println!("CacheLineAligned array addresses:");
    for (i, item) in arr.iter().enumerate() {
        println!(
            "  [{}] addr={:p}  (divisible by 64: {})",
            i,
            item as *const _,
            (item as *const _ as usize) % 64 == 0
        );
    }
    println!();
}

// offset_of macro for demonstration
macro_rules! offset_of {
    ($ty:ty, $field:ident) => {{
        let dummy = std::mem::MaybeUninit::<$ty>::uninit();
        let base = &dummy as *const _ as usize;
        let field_ptr = unsafe { &(*dummy.as_ptr()).$field as *const _ as usize };
        field_ptr - base
    }};
}

// ---------------------------------------------------------------------------
// 5. Pin demo — self-referential safety
// ---------------------------------------------------------------------------

struct SelfReferential {
    data: String,
    // Points into self.data — only valid as long as Self isn't moved
    pointer: *const u8,
}

impl SelfReferential {
    fn new(s: &str) -> Pin<Box<Self>> {
        let data = s.to_string();
        let pointer = data.as_ptr();
        Pin::new(Box::new(SelfReferential { data, pointer }))
    }

    fn first_byte(&self) -> u8 {
        // SAFETY: pointer was set to self.data.as_ptr() at construction,
        // and Pin guarantees we won't be moved, so pointer remains valid.
        unsafe { *self.pointer }
    }

    fn data(&self) -> &str {
        &self.data
    }
}

fn demo_pin() {
    println!("=== Pin: Self-Referential Struct Safety ===\n");

    let pinned = SelfReferential::new("Hello, Pin!");
    println!("data: {:?}", pinned.data());
    println!("first_byte via self-pointer: '{}' (0x{:02x})", pinned.first_byte() as char, pinned.first_byte());

    // Pin prevents this from compiling:
    // let moved = Pin::into_inner(pinned); // would need unsafe
    // Without Pin, moving `pinned` would invalidate `pointer`.
    // With Pin, the type system enforces that we can't move it.

    println!("\nPin ensures the SelfReferential struct cannot be moved,");
    println!("so the self-pointer inside remains valid for the struct's lifetime.\n");
}

// ---------------------------------------------------------------------------
// 6. Safe vs Unsafe comparison — bounds-checked vs unchecked indexing
// ---------------------------------------------------------------------------

fn safe_sum(vec: &Vec<u64>, len: usize) -> u64 {
    let mut total = 0u64;
    for i in 0..len {
        total += vec[i];
    }
    black_box(total)
}

// SAFETY: Caller guarantees i < len for all accesses.
fn unsafe_sum(vec: &Vec<u64>, len: usize) -> u64 {
    let mut total = 0u64;
    for i in 0..len {
        // SAFETY: i < len is guaranteed by the caller and loop bounds.
        total += unsafe { *vec.get_unchecked(i) };
    }
    black_box(total)
}

fn demo_safe_vs_unsafe() {
    println!("=== Safe vs Unsafe: Bounds-Checked vs Unchecked ===\n");

    const SIZE: usize = 10_000_000;
    let vec: Vec<u64> = (0..SIZE as u64).collect();

    let start = Instant::now();
    let s1 = safe_sum(&vec, SIZE);
    let safe_dur = start.elapsed();

    let start = Instant::now();
    let s2 = unsafe_sum(&vec, SIZE);
    let unsafe_dur = start.elapsed();

    println!("safe_sum   : {:?}  (result={})", safe_dur, s1);
    println!("unsafe_sum : {:?}  (result={})", unsafe_dur, s2);
    println!(
        "Unchecked is {:.1}x faster\n",
        safe_dur.as_nanos() as f64 / unsafe_dur.as_nanos().max(1) as f64
    );
}

// ---------------------------------------------------------------------------
// 7. transmute demonstration — type punning with repr(C)
// ---------------------------------------------------------------------------

#[repr(C)]
struct PacketHeader {
    magic: u32,
    version: u16,
    flags: u16,
}

fn demo_transmute() {
    println!("=== transmute: Type Punning with repr(C) ===\n");

    let header = PacketHeader { magic: 0xDEAD, version: 1, flags: 0x00FF };

    // SAFETY: PacketHeader is #[repr(C)] and we're transmuting to a byte array
    // of the same size. The layout is well-defined.
    let bytes: [u8; 8] = unsafe { std::mem::transmute(header) };
    println!("Header as bytes: {:02x?}", bytes);

    // SAFETY: Same conditions — repr(C), same size.
    let restored: PacketHeader = unsafe { std::mem::transmute(bytes) };
    println!(
        "Restored: magic=0x{:04X} version={} flags=0x{:04X}\n",
        restored.magic, restored.version, restored.flags
    );

    // Size assertion — transmute requires same-size types
    assert_eq!(std::mem::size_of::<PacketHeader>(), std::mem::size_of::<[u8; 8]>());
    println!("Size assertion passed: PacketHeader == 8 bytes\n");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("Rust for High Performance — UnsafeCell, MaybeUninit, alignment\n");
    println!("============================================================\n");

    demo_interior_mutability();
    demo_maybeuninit();
    demo_manually_drop();
    demo_alignment();
    demo_pin();
    demo_safe_vs_unsafe();
    demo_transmute();

    println!("============================================================");
    println!("All demos complete. Key takeaways:");
    println!("  1. UnsafeCell avoids RefCell overhead when you can prove single-borrows");
    println!("  2. MaybeUninit avoids memset on large buffers");
    println!("  3. Alignment matters — cache-line boundary = cache-friendly");
    println!("  4. Packed structs are for FFI only, not hot paths");
    println!("  5. Pin prevents moves on self-referential types");
    println!("  6. Always benchmark with black_box; always profile before optimizing");
    println!("  7. Wrap unsafe in safe APIs with SAFETY comments");
}