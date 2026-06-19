//! main.rs — AVL tree in safe Rust. Uses Option<Box<Node>>.

pub struct Node {
    key: i32,
    height: i32,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

fn h(n: &Option<Box<Node>>) -> i32 { n.as_ref().map_or(0, |b| b.height) }
fn bf(n: &Node) -> i32 { h(&n.left) - h(&n.right) }
fn update_height(n: &mut Node) { n.height = 1 + h(&n.left).max(h(&n.right)); }

fn rotate_left(mut n: Box<Node>) -> Box<Node> {
    let mut r = n.right.take().unwrap();
    n.right = r.left.take();
    update_height(&mut n);
    r.left = Some(n);
    update_height(&mut r);
    r
}
fn rotate_right(mut n: Box<Node>) -> Box<Node> {
    let mut l = n.left.take().unwrap();
    n.left = l.right.take();
    update_height(&mut n);
    l.right = Some(n);
    update_height(&mut l);
    l
}

fn rebalance(mut n: Box<Node>) -> Box<Node> {
    update_height(&mut n);
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

pub fn insert(n: Option<Box<Node>>, k: i32) -> Option<Box<Node>> {
    match n {
        None => Some(Box::new(Node { key: k, height: 1, left: None, right: None })),
        Some(mut b) => {
            if k < b.key      { b.left  = insert(b.left.take(),  k); }
            else if k > b.key { b.right = insert(b.right.take(), k); }
            else { return Some(b); }
            Some(rebalance(b))
        }
    }
}

pub fn verify(n: &Option<Box<Node>>) -> i32 {
    match n {
        None => 0,
        Some(b) => {
            let lh = verify(&b.left);
            let rh = verify(&b.right);
            assert!((lh - rh).abs() <= 1, "AVL violation at key={}", b.key);
            1 + lh.max(rh)
        }
    }
}

fn main() {
    let mut t: Option<Box<Node>> = None;
    for i in 1..=1000 { t = insert(t, i); }
    println!("sorted insert n=1000  → height = {} (max ≈ 14)", verify(&t));

    let mut t: Option<Box<Node>> = None;
    // pseudo-random
    let mut s: u64 = 0xdeadbeef;
    for _ in 0..10_000 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let k = (s >> 32) as i32 % 100_000;
        t = insert(t, k);
    }
    println!("random insert n=10000 → height = {} (max ≈ 19)", verify(&t));

    for (seq, label) in [
        (&[3, 2, 1][..], "LL"),
        (&[1, 2, 3][..], "RR"),
        (&[3, 1, 2][..], "LR"),
        (&[1, 3, 2][..], "RL"),
    ] {
        let mut t: Option<Box<Node>> = None;
        for &k in seq { t = insert(t, k); }
        println!("  {} insert {:?}: root = {} (expect 2)", label, seq, t.as_ref().unwrap().key);
    }
}
