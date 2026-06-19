//! main.rs — persistent BST via Arc<Node> for shared ownership.
//!
//! Uses Arc (atomic reference counting) so multiple versions can share subtrees;
//! the tree is automatically freed when all versions referencing it drop.

use std::sync::Arc;

#[derive(Debug)]
pub enum BST {
    Leaf,
    Node(i32, Arc<BST>, Arc<BST>),
}

pub fn insert(t: &Arc<BST>, k: i32) -> Arc<BST> {
    match t.as_ref() {
        BST::Leaf => Arc::new(BST::Node(k, Arc::new(BST::Leaf), Arc::new(BST::Leaf))),
        BST::Node(v, l, r) => {
            if k < *v      { Arc::new(BST::Node(*v, insert(l, k), Arc::clone(r))) }
            else if k > *v { Arc::new(BST::Node(*v, Arc::clone(l), insert(r, k))) }
            else           { Arc::clone(t) }
        }
    }
}

pub fn contains(t: &BST, k: i32) -> bool {
    match t {
        BST::Leaf => false,
        BST::Node(v, l, r) => {
            if k < *v      { contains(l, k) }
            else if k > *v { contains(r, k) }
            else           { true }
        }
    }
}

pub fn count(t: &BST) -> usize {
    match t {
        BST::Leaf => 0,
        BST::Node(_, l, r) => 1 + count(l) + count(r),
    }
}

fn main() {
    let mut t1 = Arc::new(BST::Leaf);
    for k in [0, 10, 20, 30, 40, 50, 60, 70] {
        t1 = insert(&t1, k);
    }
    let t2 = insert(&t1, 25);

    println!("t1 nodes: {}, t2 nodes: {}", count(&t1), count(&t2));
    println!("t1 contains 25: {}", contains(&t1, 25));
    println!("t2 contains 25: {}", contains(&t2, 25));
    println!("Both trees coexist; t1 unmodified.");
}
