// Phase 13, Lesson 07 — Atomics, CAS, ABA Problem
// Demonstrates: atomic counters, lock-free Treiber stack via CAS,
// the ABA problem (conceptual in Rust), and tagged-pointer ABA prevention.
//
// Compile:  rustc main.rs -o atomic_lesson
// Run:      ./atomic_lesson

use std::marker::PhantomData;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

// ============================================================================
// Step 1: Atomic Counter
// ============================================================================
// Compare AtomicUsize::fetch_add (FAA) against Mutex-protected increment.

struct AtomicCounter(AtomicUsize);

impl AtomicCounter {
    fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    fn increment(&self) -> usize {
        self.0.fetch_add(1, Ordering::Relaxed)
    }

    fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

struct MutexCounter {
    inner: Mutex<usize>,
}

impl MutexCounter {
    fn new() -> Self {
        Self { inner: Mutex::new(0) }
    }

    fn increment(&self) {
        *self.inner.lock().unwrap() += 1;
    }

    fn get(&self) -> usize {
        *self.inner.lock().unwrap()
    }
}

fn bench_counter() {
    const N: usize = 1_000_000;
    const THREADS: usize = 8;

    let atomic = AtomicCounter::new();
    let start = Instant::now();
    thread::scope(|s| {
        for _ in 0..THREADS {
            s.spawn(|| {
                for _ in 0..N {
                    atomic.increment();
                }
            });
        }
    });
    let atomic_dur = start.elapsed();
    println!("  Atomic counter: {} in {:?} (final={})",
        N * THREADS, atomic_dur, atomic.get());

    let mutex = MutexCounter::new();
    let start = Instant::now();
    thread::scope(|s| {
        for _ in 0..THREADS {
            s.spawn(|| {
                for _ in 0..N {
                    mutex.increment();
                }
            });
        }
    });
    let mutex_dur = start.elapsed();
    println!("  Mutex counter:  {} in {:?} (final={})",
        N * THREADS, mutex_dur, mutex.get());

    println!("  Speedup: {:.2}x", mutex_dur.as_secs_f64() / atomic_dur.as_secs_f64());
    println!("  Note: Ordering::Relaxed is safe — no cross-thread ordering needed.");
}

// ============================================================================
// Step 2: CAS-based Lock-Free Stack (Treiber Stack)
// ============================================================================

struct Node<T> {
    value: T,
    next: *mut Node<T>,
}

unsafe impl<T: Send> Send for Node<T> {}
unsafe impl<T: Send> Sync for Node<T> {}

struct LockFreeStack<T> {
    head: AtomicPtr<Node<T>>,
}

unsafe impl<T: Send> Send for LockFreeStack<T> {}
unsafe impl<T: Send> Sync for LockFreeStack<T> {}

impl<T> Drop for LockFreeStack<T> {
    fn drop(&mut self) {
        let mut ptr = self.head.load(Ordering::Relaxed);
        while !ptr.is_null() {
            let node = unsafe { Box::from_raw(ptr) };
            ptr = node.next;
        }
    }
}

impl<T> LockFreeStack<T> {
    fn new() -> Self {
        Self { head: AtomicPtr::new(std::ptr::null_mut()) }
    }

    fn push(&self, value: T) {
        let node = Box::into_raw(Box::new(Node {
            value,
            next: std::ptr::null_mut(),
        }));
        loop {
            let head = self.head.load(Ordering::Acquire);
            unsafe { (*node).next = head; }
            if self.head
                .compare_exchange(head, node, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    fn pop(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            if head.is_null() {
                return None;
            }
            let next = unsafe { (*head).next };
            if self.head
                .compare_exchange(head, next, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                let node = unsafe { Box::from_raw(head) };
                return Some(node.value);
            }
        }
    }
}

fn test_lockfree_stack() {
    let stack = Arc::new(LockFreeStack::new());
    let values: Vec<i32> = (0..2000).collect();

    let mut handles = vec![];
    for chunk in values.chunks(500) {
        let local = chunk.to_vec();
        let stack_clone = stack.clone();
        handles.push(thread::spawn(move || {
            for &v in &local {
                stack_clone.push(v);
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let mut popped = Vec::new();
    while let Some(v) = stack.pop() {
        popped.push(v);
    }

    popped.sort();
    assert_eq!(popped, values);
    println!("  Lock-free stack: {} values pushed + popped correctly", popped.len());
}

// ============================================================================
// Step 3: The ABA Problem (Conceptual in Rust)
// ============================================================================
//
// The ABA problem: CAS checks *address equality* on a pointer. If the address
// cycles A → B → A, CAS cannot distinguish "nothing happened" from
// "something happened and was undone."
//
// In C++ with manual memory management:
//   T1: head = load(A)           → plans CAS(&head, A, A->next=B)
//   T1: (preempted)
//   T2: pop(A), free(A)          → head = B
//   T2: pop(B), free(B)          → head = null
//   T2: alloc(C) at A's addr     → head = C (C at same address as A!)
//   T1: CAS(&head, A, B)         → SUCCEEDS (head == A's address)
//                                → head = B, BUT B WAS FREED!
//
// Rust's borrow checker prevents this exact scenario: you cannot call
// Box::from_raw (free) while any raw pointer to the memory still exists,
// at least not without extensive unsafe that recreates C++ semantics.
//
// The C++ file (main.cpp) in this lesson provides a deterministic,
// runnable demonstration of the ABA bug using a node-recycling allocator.
// Run it to see the corruption in action.

fn aba_conceptual_demo() {
    println!("  Rust's borrow checker prevents the simplest ABA scenario.");
    println!("  See main.cpp for a deterministic ABA demonstration with");
    println!("  a node-recycling allocator that reuses addresses.");
    println!();
    println!("  Key insight: CAS only checks bit-pattern equality.");
    println!("  If pointer address matches but memory has been recycled,");
    println!("  the data structure is corrupted.");
}

// ============================================================================
// Step 4: ABA Solution — Tagged Pointer
// ============================================================================
// Embed a version counter in the lowest 3 bits of the AtomicUsize.
// User-space pointers are at least 8-byte aligned (bottom 3 bits are 0),
// so we can safely co-locate a 3-bit tag.
//
// Each successful CAS increments the tag. Even if the pointer value cycles
// back to an old address, the tag differs → CAS fails → retry.

const TAG_BITS: usize = 3;
const TAG_MASK: usize = (1 << TAG_BITS) - 1;
const PTR_MASK: usize = !TAG_MASK;

fn pack<T>(ptr: *mut T, tag: usize) -> usize {
    (ptr as usize & PTR_MASK) | (tag & TAG_MASK)
}

fn unpack<T>(val: usize) -> (*mut T, usize) {
    let ptr = (val & PTR_MASK) as *mut T;
    let tag = val & TAG_MASK;
    (ptr, tag)
}

struct TaggedStack<T> {
    head: AtomicUsize,
    _phantom: PhantomData<T>,
}

unsafe impl<T: Send> Send for TaggedStack<T> {}
unsafe impl<T: Send> Sync for TaggedStack<T> {}

impl<T> Drop for TaggedStack<T> {
    fn drop(&mut self) {
        let (mut ptr, _): (*mut Node<T>, _) = unpack(self.head.load(Ordering::Relaxed));
        while !ptr.is_null() {
            let node = unsafe { Box::from_raw(ptr) };
            ptr = node.next;
        }
    }
}

impl<T> TaggedStack<T> {
    fn new() -> Self {
        Self {
            head: AtomicUsize::new(pack::<Node<T>>(std::ptr::null_mut(), 0)),
            _phantom: PhantomData,
        }
    }

    fn push(&self, value: T) {
        let node = Box::into_raw(Box::new(Node {
            value,
            next: std::ptr::null_mut(),
        }));
        loop {
            let head_val = self.head.load(Ordering::Acquire);
            let (head_ptr, _tag): (*mut Node<T>, _) = unpack(head_val);
            unsafe { (*node).next = head_ptr; }
            let new_val = pack(node, (_tag + 1) & TAG_MASK);
            if self.head
                .compare_exchange(head_val, new_val, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    fn pop(&self) -> Option<T> {
        loop {
            let head_val = self.head.load(Ordering::Acquire);
            let (head_ptr, tag): (*mut Node<T>, _) = unpack(head_val);
            if head_ptr.is_null() {
                return None;
            }
            let next_ptr = unsafe { (*head_ptr).next };
            let new_val = pack(next_ptr, (tag + 1) & TAG_MASK);
            if self.head
                .compare_exchange(head_val, new_val, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                let node = unsafe { Box::from_raw(head_ptr) };
                return Some(node.value);
            }
        }
    }
}

fn test_tagged_stack() {
    let stack = TaggedStack::new();
    let n = 10_000;

    thread::scope(|s| {
        s.spawn(|| {
            for i in 0..n {
                stack.push(i);
            }
        });
        s.spawn(|| {
            for i in 0..n {
                stack.push(i + n);
            }
        });
    });

    let mut count = 0;
    while stack.pop().is_some() {
        count += 1;
    }
    assert_eq!(count, 2 * n);
    println!("  Tagged-pointer stack: {} values pushed + popped correctly", count);
    println!("  (3-bit tag prevents ABA — each CAS increments version)");
}

// ============================================================================
// Main — run all four steps sequentially
// ============================================================================

fn main() {
    println!("=== Phase 13.07: Atomics, CAS, ABA Problem ===\n");

    println!("--- Step 1: Atomic Counter (FAA vs Mutex) ---");
    println!("  8 threads x 1,000,000 increments each");
    bench_counter();

    println!();
    println!("--- Step 2: Lock-Free Stack (Treiber Stack) ---");
    println!("  CAS-based push/pop on AtomicPtr<Node<T>>");
    test_lockfree_stack();

    println!();
    println!("--- Step 3: The ABA Problem ---");
    aba_conceptual_demo();

    println!();
    println!("--- Step 4: ABA Solution — Tagged Pointer ---");
    println!("  Version counter in bottom 3 AtomicUsize bits");
    test_tagged_stack();

    println!();
    println!("=== All steps completed. ===");
}
