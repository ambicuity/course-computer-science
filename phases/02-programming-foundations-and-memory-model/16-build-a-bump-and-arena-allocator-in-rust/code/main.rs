//! Bump allocator and typed arena in Rust.
//!
//! Build: rustc -O main.rs -o m && ./m
//!
//! The Bump implementation uses unsafe internally (to extend a slot's lifetime
//! to the arena's), but exposes a fully-safe public API.

#![allow(clippy::mut_from_ref)]

use std::cell::{Cell, RefCell};
use std::mem;
use std::ptr;

// ── Untyped Bump ──────────────────────────────────────────────────

pub struct Bump {
    buf: Box<[u8]>,
    offset: Cell<usize>,
}

impl Bump {
    pub fn with_capacity(n: usize) -> Self {
        Bump {
            buf: vec![0u8; n].into_boxed_slice(),
            offset: Cell::new(0),
        }
    }

    pub fn alloc<T>(&self, value: T) -> &mut T {
        let layout = std::alloc::Layout::new::<T>();
        let off = self.offset.get();
        let aligned = (off + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        if end > self.buf.len() {
            panic!("Bump exhausted: requested {} bytes, only {} left",
                   layout.size(), self.buf.len() - off);
        }
        self.offset.set(end);
        unsafe {
            let ptr = self.buf.as_ptr().add(aligned) as *mut T;
            ptr::write(ptr, value);
            &mut *ptr
        }
    }

    pub fn bytes_used(&self) -> usize { self.offset.get() }

    /// Reset the allocator. UNSAFE because outstanding references are now invalid.
    /// (In production, the borrow checker prevents calling this while references live.)
    pub fn reset(&mut self) { self.offset.set(0); }
}

// ── Typed Arena (drop-aware) ──────────────────────────────────────

pub struct Arena<T> {
    chunks: RefCell<Vec<Vec<T>>>,
}

impl<T> Arena<T> {
    pub fn new() -> Self { Arena { chunks: RefCell::new(vec![Vec::with_capacity(16)]) } }

    pub fn alloc(&self, value: T) -> &mut T {
        let mut chunks = self.chunks.borrow_mut();
        let last = chunks.last_mut().unwrap();
        if last.len() == last.capacity() {
            let new_cap = last.capacity() * 2;
            chunks.push(Vec::with_capacity(new_cap));
        }
        let last = chunks.last_mut().unwrap();
        last.push(value);
        let idx = last.len() - 1;
        // SAFETY: the Vec never reallocates once filled (we always push to a
        // freshly-grown one) and the arena outlives every returned reference.
        unsafe {
            let ptr: *mut T = last.as_mut_ptr().add(idx);
            &mut *ptr
        }
    }

    pub fn len(&self) -> usize {
        self.chunks.borrow().iter().map(|c| c.len()).sum()
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self { Self::new() }
}

// ── Demo: a small AST built from arena references ───────────────

struct Node<'a> {
    name: String,
    children: Vec<&'a Node<'a>>,
}

fn print_tree(n: &Node, depth: usize) {
    println!("{}- {}", "  ".repeat(depth), n.name);
    for c in &n.children {
        print_tree(c, depth + 1);
    }
}

struct Dropper {
    id: u32,
}

impl Drop for Dropper {
    fn drop(&mut self) {
        println!("  ↓ Dropping Dropper({})", self.id);
    }
}

fn main() {
    println!("== Bump allocator ==");
    let bump = Bump::with_capacity(4096);
    let a: &mut i32 = bump.alloc(42);
    let b: &mut i32 = bump.alloc(7);
    let c: &mut String = bump.alloc(String::from("hello"));
    println!("  a = {} (at {:p})", a, a);
    println!("  b = {} (at {:p})", b, b);
    println!("  c = {:?} (at {:p})", c, c);
    println!("  bytes used: {}", bump.bytes_used());

    println!("\n== Typed Arena ==");
    let arena: Arena<Node> = Arena::new();
    let leaf1 = arena.alloc(Node { name: "leaf1".into(), children: vec![] });
    let leaf2 = arena.alloc(Node { name: "leaf2".into(), children: vec![] });
    let branch = arena.alloc(Node { name: "branch".into(), children: vec![leaf1, leaf2] });
    let root = arena.alloc(Node { name: "root".into(), children: vec![branch] });
    print_tree(root, 0);
    println!("  arena holds {} nodes", arena.len());

    println!("\n== Arena drops all values when it dies ==");
    {
        let drop_arena: Arena<Dropper> = Arena::new();
        for i in 0..3 {
            drop_arena.alloc(Dropper { id: i });
        }
        println!("  3 Droppers allocated; arena going out of scope now");
    } // 3 Droppers dropped here, one shot

    println!("\n== 1000 contiguous i32s from a Bump ==");
    let big_bump = Bump::with_capacity(1024 * 16);
    let first_addr;
    {
        let r: &mut i32 = big_bump.alloc(0);
        first_addr = r as *const _ as usize;
    }
    for i in 1..1000 {
        big_bump.alloc(i as i32);
    }
    println!("  first addr = 0x{:x}, used = {} bytes", first_addr, big_bump.bytes_used());
}
