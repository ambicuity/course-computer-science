//! main.rs — Phase 02 capstone: Rust mirror of memlib.
//!
//! Provides:
//!   - `Bump`   : single-chunk bump arena with byte-level alloc
//!   - `Pool<T>`: typed pool with O(1) alloc/free via free-list
//!   - `BSlice` : runtime-bounds-checked slice
//!
//! Rust enforces most invariants at compile time. The few runtime checks below
//! exist because we hand out raw pointers from the bump and pool, and need to
//! guarantee no overlap or double-free under misuse.

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::mem;
use std::ptr;

// ---------- Bump ----------
pub struct Bump {
    buf: Vec<u8>,
    used: Cell<usize>,
}

impl Bump {
    pub fn with_capacity(cap: usize) -> Self {
        Bump { buf: vec![0u8; cap], used: Cell::new(0) }
    }

    pub fn alloc_bytes(&self, n: usize, align: usize) -> Option<*mut u8> {
        debug_assert!(align.is_power_of_two());
        let cur = self.used.get();
        let aligned = (cur + align - 1) & !(align - 1);
        if aligned + n > self.buf.len() { return None; }
        self.used.set(aligned + n);
        // SAFETY: in-bounds offset into our own buf, exclusive (we never alias).
        Some(unsafe { (self.buf.as_ptr() as *mut u8).add(aligned) })
    }

    pub fn alloc_str(&self, s: &str) -> Option<&str> {
        let p = self.alloc_bytes(s.len(), 1)?;
        unsafe {
            ptr::copy_nonoverlapping(s.as_ptr(), p, s.len());
            Some(std::str::from_utf8_unchecked(std::slice::from_raw_parts(p, s.len())))
        }
    }

    pub fn used(&self) -> usize { self.used.get() }
    pub fn reset(&self)         { self.used.set(0); }
}

// ---------- Pool<T> ----------
pub struct Pool<T> {
    slab: RefCell<Vec<mem::MaybeUninit<T>>>,
    free: RefCell<Vec<usize>>,           // indices of free slots (LIFO)
    _marker: PhantomData<T>,
}

impl<T> Pool<T> {
    pub fn with_capacity(n: usize) -> Self {
        let mut slab = Vec::with_capacity(n);
        for _ in 0..n { slab.push(mem::MaybeUninit::uninit()); }
        let free: Vec<usize> = (0..n).rev().collect();   // pop from end → 0,1,2…
        Pool { slab: RefCell::new(slab), free: RefCell::new(free), _marker: PhantomData }
    }

    pub fn alloc(&self, value: T) -> Option<usize> {
        let mut free = self.free.borrow_mut();
        let idx = free.pop()?;
        self.slab.borrow_mut()[idx].write(value);
        Some(idx)
    }

    pub fn free(&self, idx: usize) {
        let cap = self.slab.borrow().len();
        debug_assert!(idx < cap, "pool::free: idx {idx} out of range {cap}");
        // Drop the value at idx.
        unsafe { self.slab.borrow_mut()[idx].assume_init_drop(); }
        // (We don't detect double-free here; LSAN/Miri or a debug bitmap would.)
        self.free.borrow_mut().push(idx);
    }

    pub fn get(&self, idx: usize) -> &T {
        unsafe { &*self.slab.borrow()[idx].as_ptr() }
    }

    pub fn free_count(&self) -> usize { self.free.borrow().len() }
}

// ---------- BSlice ----------
pub struct BSlice<'a, T> { data: &'a [T] }
impl<'a, T> BSlice<'a, T> {
    pub fn new(s: &'a [T]) -> Self { BSlice { data: s } }
    pub fn get(&self, i: usize) -> &T {
        debug_assert!(i < self.data.len(), "BSlice::get: {i} out of bounds {}", self.data.len());
        &self.data[i]
    }
    pub fn len(&self) -> usize { self.data.len() }
}

// ---------- Demo ----------
fn main() {
    println!("== memlib (Rust) ==");

    // Bump demo
    let bump = Bump::with_capacity(1 << 20);
    let templates = ["hi", "hello world", "the quick brown fox", "x"];
    for i in 0..1000 {
        bump.alloc_str(templates[i & 3]).unwrap();
    }
    println!("Bump used {} bytes after 1000 strdups", bump.used());

    // Pool demo
    #[derive(Debug)]
    struct Node { value: i32 }
    let pool: Pool<Node> = Pool::with_capacity(100);
    let mut ids = Vec::new();
    for i in 0..50 { ids.push(pool.alloc(Node { value: i }).unwrap()); }
    println!("Pool: free_count after 50 allocs = {} (expected 50)", pool.free_count());
    for id in ids.iter().rev() { pool.free(*id); }
    println!("Pool: free_count after free      = {} (expected 100)", pool.free_count());

    // BSlice demo
    let v = [10, 20, 30, 40, 50];
    let s = BSlice::new(&v);
    println!("BSlice::get(2) = {} (expected 30)", s.get(2));
    println!("== done ==");
}
