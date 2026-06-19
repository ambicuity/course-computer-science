//! Phase 3 Capstone Library — generic DS surface.
//!
//! Add this to a Cargo project as `src/lib.rs` and import the types.
//!
//!   use phase3_ds::{DynVec, LinkedList, MinHeap, AvlSet, Collection};

pub trait Collection<T> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
}

pub struct DynVec<T> { data: Vec<T> }
impl<T> DynVec<T> {
    pub fn new() -> Self { Self { data: Vec::new() } }
    pub fn push(&mut self, v: T) { self.data.push(v); }
    pub fn pop(&mut self) -> Option<T> { self.data.pop() }
    pub fn get(&self, i: usize) -> Option<&T> { self.data.get(i) }
    pub fn as_slice(&self) -> &[T] { &self.data }
}
impl<T> Collection<T> for DynVec<T> { fn len(&self) -> usize { self.data.len() } }

pub struct LinkedList<T> { head: Option<Box<LNode<T>>>, len: usize }
struct LNode<T> { value: T, next: Option<Box<LNode<T>>> }
impl<T> LinkedList<T> {
    pub fn new() -> Self { Self { head: None, len: 0 } }
    pub fn push_front(&mut self, v: T) {
        self.head = Some(Box::new(LNode { value: v, next: self.head.take() }));
        self.len += 1;
    }
    pub fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|n| { self.head = n.next; self.len -= 1; n.value })
    }
}
impl<T> Collection<T> for LinkedList<T> { fn len(&self) -> usize { self.len } }

pub struct MinHeap<T: Ord> { data: Vec<T> }
impl<T: Ord> MinHeap<T> {
    pub fn new() -> Self { Self { data: Vec::new() } }
    pub fn push(&mut self, v: T) {
        self.data.push(v);
        let mut i = self.data.len() - 1;
        while i > 0 {
            let p = (i - 1) / 2;
            if self.data[p] <= self.data[i] { break; }
            self.data.swap(p, i); i = p;
        }
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.data.is_empty() { return None; }
        let last = self.data.len() - 1;
        self.data.swap(0, last);
        let v = self.data.pop();
        let mut i = 0; let n = self.data.len();
        loop {
            let l = 2 * i + 1; let r = 2 * i + 2;
            let mut smallest = i;
            if l < n && self.data[l] < self.data[smallest] { smallest = l; }
            if r < n && self.data[r] < self.data[smallest] { smallest = r; }
            if smallest == i { break; }
            self.data.swap(i, smallest); i = smallest;
        }
        v
    }
    pub fn peek(&self) -> Option<&T> { self.data.first() }
}
impl<T: Ord> Collection<T> for MinHeap<T> { fn len(&self) -> usize { self.data.len() } }

// AvlSet abridged (see main.rs for full source with rotations).
