//! Phase 3 Capstone — a generic Rust DS library exercising Phase 3 in one binary.
//!
//! Includes: DynVec<T>, LinkedList<T>, MinHeap<T>, AvlSet<T>.
//! All generic over the value type with appropriate trait bounds.
//! Invariants are checked via debug_assert! after every mutation.

pub trait Collection<T> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
}

// ============================================================
// DynVec<T>
// ============================================================
pub struct DynVec<T> { data: Vec<T> }

impl<T> DynVec<T> {
    pub fn new() -> Self { DynVec { data: Vec::new() } }
    pub fn push(&mut self, v: T) { self.data.push(v); }
    pub fn pop(&mut self) -> Option<T> { self.data.pop() }
    pub fn get(&self, i: usize) -> Option<&T> { self.data.get(i) }
}
impl<T> Collection<T> for DynVec<T> { fn len(&self) -> usize { self.data.len() } }

// ============================================================
// LinkedList<T>
// ============================================================
pub struct LinkedList<T> { head: Option<Box<Node<T>>>, len: usize }
struct Node<T> { value: T, next: Option<Box<Node<T>>> }

impl<T> LinkedList<T> {
    pub fn new() -> Self { LinkedList { head: None, len: 0 } }
    pub fn push_front(&mut self, v: T) {
        self.head = Some(Box::new(Node { value: v, next: self.head.take() }));
        self.len += 1;
    }
    pub fn pop_front(&mut self) -> Option<T> {
        self.head.take().map(|node| { self.head = node.next; self.len -= 1; node.value })
    }
}
impl<T> Collection<T> for LinkedList<T> { fn len(&self) -> usize { self.len } }

// ============================================================
// MinHeap<T>
// ============================================================
pub struct MinHeap<T: Ord> { data: Vec<T> }

impl<T: Ord> MinHeap<T> {
    pub fn new() -> Self { MinHeap { data: Vec::new() } }

    pub fn push(&mut self, v: T) {
        self.data.push(v);
        let mut i = self.data.len() - 1;
        while i > 0 {
            let p = (i - 1) / 2;
            if self.data[p] <= self.data[i] { break; }
            self.data.swap(p, i);
            i = p;
        }
        debug_assert!(self.invariant());
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.data.is_empty() { return None; }
        let last = self.data.len() - 1;
        self.data.swap(0, last);
        let v = self.data.pop();
        self.sift_down(0);
        debug_assert!(self.invariant());
        v
    }

    fn sift_down(&mut self, mut i: usize) {
        let n = self.data.len();
        loop {
            let l = 2 * i + 1; let r = 2 * i + 2;
            let mut smallest = i;
            if l < n && self.data[l] < self.data[smallest] { smallest = l; }
            if r < n && self.data[r] < self.data[smallest] { smallest = r; }
            if smallest == i { return; }
            self.data.swap(i, smallest);
            i = smallest;
        }
    }

    fn invariant(&self) -> bool {
        for i in 1..self.data.len() {
            if self.data[(i - 1) / 2] > self.data[i] { return false; }
        }
        true
    }
}

// ============================================================
// AvlSet<T>
// ============================================================
struct AvlNode<T> {
    key: T, height: i32,
    left: Option<Box<AvlNode<T>>>, right: Option<Box<AvlNode<T>>>,
}

pub struct AvlSet<T: Ord> { root: Option<Box<AvlNode<T>>>, len: usize }

fn h<T>(n: &Option<Box<AvlNode<T>>>) -> i32 { n.as_ref().map_or(0, |b| b.height) }
fn bf<T>(n: &AvlNode<T>) -> i32 { h(&n.left) - h(&n.right) }
fn update_h<T>(n: &mut AvlNode<T>) { n.height = 1 + h(&n.left).max(h(&n.right)); }

fn rotate_left<T>(mut n: Box<AvlNode<T>>) -> Box<AvlNode<T>> {
    let mut r = n.right.take().unwrap();
    n.right = r.left.take();
    update_h(&mut n);
    r.left = Some(n);
    update_h(&mut r);
    r
}
fn rotate_right<T>(mut n: Box<AvlNode<T>>) -> Box<AvlNode<T>> {
    let mut l = n.left.take().unwrap();
    n.left = l.right.take();
    update_h(&mut n);
    l.right = Some(n);
    update_h(&mut l);
    l
}
fn rebalance<T>(mut n: Box<AvlNode<T>>) -> Box<AvlNode<T>> {
    update_h(&mut n);
    let b = bf(&n);
    if b > 1 {
        if bf(n.left.as_ref().unwrap()) < 0 {
            n.left = Some(rotate_left(n.left.take().unwrap()));
        }
        return rotate_right(n);
    }
    if b < -1 {
        if bf(n.right.as_ref().unwrap()) > 0 {
            n.right = Some(rotate_right(n.right.take().unwrap()));
        }
        return rotate_left(n);
    }
    n
}

fn insert_rec<T: Ord>(n: Option<Box<AvlNode<T>>>, k: T) -> (Option<Box<AvlNode<T>>>, bool) {
    match n {
        None => (Some(Box::new(AvlNode { key: k, height: 1, left: None, right: None })), true),
        Some(mut b) => {
            let inserted;
            if k < b.key {
                let (l, ins) = insert_rec(b.left.take(), k);
                b.left = l; inserted = ins;
            } else if k > b.key {
                let (r, ins) = insert_rec(b.right.take(), k);
                b.right = r; inserted = ins;
            } else {
                inserted = false;
            }
            if !inserted { return (Some(b), false); }
            (Some(rebalance(b)), true)
        }
    }
}

fn contains_rec<T: Ord>(n: &Option<Box<AvlNode<T>>>, k: &T) -> bool {
    let mut cur = n.as_deref();
    while let Some(b) = cur {
        match k.cmp(&b.key) {
            std::cmp::Ordering::Less => cur = b.left.as_deref(),
            std::cmp::Ordering::Greater => cur = b.right.as_deref(),
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

fn verify_rec<T>(n: &Option<Box<AvlNode<T>>>) -> (bool, i32) {
    match n {
        None => (true, 0),
        Some(b) => {
            let (lok, lh) = verify_rec(&b.left);
            let (rok, rh) = verify_rec(&b.right);
            (lok && rok && (lh - rh).abs() <= 1, 1 + lh.max(rh))
        }
    }
}

impl<T: Ord> AvlSet<T> {
    pub fn new() -> Self { AvlSet { root: None, len: 0 } }
    pub fn insert(&mut self, k: T) -> bool {
        let (new_root, inserted) = insert_rec(self.root.take(), k);
        self.root = new_root;
        if inserted { self.len += 1; }
        debug_assert!(verify_rec(&self.root).0);
        inserted
    }
    pub fn contains(&self, k: &T) -> bool { contains_rec(&self.root, k) }
    pub fn height(&self) -> i32 { h(&self.root) }
}
impl<T: Ord> Collection<T> for AvlSet<T> { fn len(&self) -> usize { self.len } }

// ============================================================
// Demo
// ============================================================
fn pseudo(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *seed >> 32
}

fn main() {
    println!("== Phase 3 Capstone — generic DS library ==\n");

    let mut v: DynVec<i32> = DynVec::new();
    for i in 0..100 { v.push(i); }
    println!("DynVec<i32>: len={}, get(50) = {:?}", v.len(), v.get(50));

    let mut l: LinkedList<&str> = LinkedList::new();
    for s in ["a", "b", "c"] { l.push_front(s); }
    println!("LinkedList<&str>: len={}, pop_front = {:?}", l.len(), l.pop_front());

    let mut h: MinHeap<i32> = MinHeap::new();
    let mut seed: u64 = 42;
    for _ in 0..1000 { h.push((pseudo(&mut seed) % 10_000) as i32); }
    let first10: Vec<i32> = (0..10).map(|_| h.pop().unwrap()).collect();
    println!("MinHeap<i32>: smallest 10 = {:?}", first10);
    println!("  is sorted ascending: {}", first10.windows(2).all(|w| w[0] <= w[1]));

    let mut s: AvlSet<i32> = AvlSet::new();
    let mut seed: u64 = 12345;
    for _ in 0..10_000 { s.insert((pseudo(&mut seed) % 100_000) as i32); }
    let (ok, _) = verify_rec(&s.root);
    println!("AvlSet<i32>: len={}, height={}, AVL invariant: {}", s.len(), s.height(), ok);

    println!("\nAll invariants verified.");
}
