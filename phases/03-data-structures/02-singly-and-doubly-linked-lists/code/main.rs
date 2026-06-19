//! main.rs — Singly linked list (safe), Doubly linked list (raw-pointer unsafe).
//!
//! Lesson: Rust's borrow checker makes a safe SLL clean (each node owns its `next`
//! via `Option<Box<Node>>`) but a safe DLL gymnastic (prev and next alias one
//! another). Real Rust DLLs use raw pointers + `unsafe`, exactly like C — the
//! safety comes from the public API enforcing single ownership at the list level.

// =========================== Safe SLL ===========================

pub struct SList<T> {
    head: Option<Box<SNode<T>>>,
    len: usize,
}
struct SNode<T> { data: T, next: Option<Box<SNode<T>>> }

impl<T> SList<T> {
    pub fn new() -> Self { SList { head: None, len: 0 } }

    pub fn push_front(&mut self, data: T) {
        let node = Box::new(SNode { data, next: self.head.take() });
        self.head = Some(node);
        self.len += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|n| {
            self.head = n.next;
            self.len -= 1;
            n.data
        })
    }

    pub fn iter(&self) -> SIter<'_, T> { SIter { cur: self.head.as_deref() } }
    pub fn len(&self) -> usize { self.len }
}

pub struct SIter<'a, T> { cur: Option<&'a SNode<T>> }
impl<'a, T> Iterator for SIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.cur.map(|n| { self.cur = n.next.as_deref(); &n.data })
    }
}

// =========================== Unsafe DLL ===========================
// One owner of the list owns all nodes; raw pointers used internally for prev/next.

use std::ptr::NonNull;
use std::marker::PhantomData;

pub struct DList<T> {
    head: Option<NonNull<DNode<T>>>,
    tail: Option<NonNull<DNode<T>>>,
    len: usize,
    _marker: PhantomData<Box<DNode<T>>>,
}
struct DNode<T> {
    data: T,
    prev: Option<NonNull<DNode<T>>>,
    next: Option<NonNull<DNode<T>>>,
}

impl<T> DList<T> {
    pub fn new() -> Self {
        DList { head: None, tail: None, len: 0, _marker: PhantomData }
    }

    pub fn push_back(&mut self, data: T) {
        let node = Box::leak(Box::new(DNode { data, prev: self.tail, next: None }));
        let new_tail = unsafe { NonNull::new_unchecked(node as *mut _) };
        match self.tail {
            Some(old_tail) => unsafe { (*old_tail.as_ptr()).next = Some(new_tail); }
            None => self.head = Some(new_tail),
        }
        self.tail = Some(new_tail);
        self.len += 1;
    }

    pub fn pop_back(&mut self) -> Option<T> {
        self.tail.map(|tail| unsafe {
            let boxed = Box::from_raw(tail.as_ptr());
            self.tail = boxed.prev;
            match self.tail {
                Some(p) => (*p.as_ptr()).next = None,
                None => self.head = None,
            }
            self.len -= 1;
            boxed.data
        })
    }

    pub fn len(&self) -> usize { self.len }

    pub fn iter(&self) -> DIter<'_, T> {
        DIter { cur: self.head, _marker: PhantomData }
    }
}

impl<T> Drop for DList<T> {
    fn drop(&mut self) { while self.pop_back().is_some() {} }
}

pub struct DIter<'a, T> {
    cur: Option<NonNull<DNode<T>>>,
    _marker: PhantomData<&'a DNode<T>>,
}
impl<'a, T> Iterator for DIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        self.cur.map(|p| unsafe {
            self.cur = (*p.as_ptr()).next;
            &(*p.as_ptr()).data
        })
    }
}

// =========================== Demo ===========================

fn main() {
    println!("== Safe SLL (Option<Box<Node>>) ==");
    let mut s: SList<i32> = SList::new();
    for i in [3, 2, 1] { s.push_front(i); }
    println!("  after 3 push_fronts: {:?}", s.iter().collect::<Vec<_>>());
    println!("  pop_front -> {:?}", s.pop_front());
    println!("  len = {}", s.len());

    println!("\n== Unsafe DLL (NonNull<DNode<T>>) ==");
    let mut d: DList<i32> = DList::new();
    for i in [10, 20, 30, 40] { d.push_back(i); }
    println!("  after push_back × 4: {:?}", d.iter().collect::<Vec<_>>());
    println!("  pop_back -> {:?}", d.pop_back());
    println!("  pop_back -> {:?}", d.pop_back());
    println!("  remaining: {:?}", d.iter().collect::<Vec<_>>());
    println!("  len = {}", d.len());
    println!("\n== done ==");
}
