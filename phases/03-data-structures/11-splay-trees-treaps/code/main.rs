//! main.rs — Treap in Rust (splay is awkward in safe Rust due to mutation-through-aliasing).

pub struct Treap {
    key: i32,
    prio: u32,
    left: Option<Box<Treap>>,
    right: Option<Box<Treap>>,
}

fn xorshift(s: &mut u64) -> u32 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    (*s as u32) & 0x7fffffff
}

fn rotate_left(mut n: Box<Treap>) -> Box<Treap> {
    let mut r = n.right.take().unwrap();
    n.right = r.left.take();
    r.left = Some(n);
    r
}
fn rotate_right(mut n: Box<Treap>) -> Box<Treap> {
    let mut l = n.left.take().unwrap();
    n.left = l.right.take();
    l.right = Some(n);
    l
}

fn insert(n: Option<Box<Treap>>, k: i32, seed: &mut u64) -> Box<Treap> {
    let mut n = match n {
        None => return Box::new(Treap { key: k, prio: xorshift(seed), left: None, right: None }),
        Some(b) => b,
    };
    if k < n.key {
        n.left = Some(insert(n.left.take(), k, seed));
        if n.left.as_ref().unwrap().prio > n.prio {
            n = rotate_right(n);
        }
    } else if k > n.key {
        n.right = Some(insert(n.right.take(), k, seed));
        if n.right.as_ref().unwrap().prio > n.prio {
            n = rotate_left(n);
        }
    }
    n
}

fn height(n: &Option<Box<Treap>>) -> i32 {
    match n {
        None => 0,
        Some(b) => 1 + height(&b.left).max(height(&b.right)),
    }
}

fn main() {
    let mut t: Option<Box<Treap>> = None;
    let mut seed: u64 = 0xdeadbeef;
    for i in 1..=10_000 {
        t = Some(insert(t.take(), i, &mut seed));
    }
    println!("Treap height after sorted insert 1..10000: {}  (expected ~28)", height(&t));
}
