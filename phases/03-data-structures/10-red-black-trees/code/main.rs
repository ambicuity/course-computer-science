//! main.rs — Left-Leaning Red-Black tree in Rust.
//! Note: Rust's std::collections::BTreeMap is what you'd actually use in production.

const RED: bool = true;
const BLACK: bool = false;

pub struct Node {
    key: i32,
    color: bool,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

fn is_red(n: &Option<Box<Node>>) -> bool {
    n.as_ref().map_or(false, |b| b.color == RED)
}

fn rotate_left(mut n: Box<Node>) -> Box<Node> {
    let mut r = n.right.take().unwrap();
    n.right = r.left.take();
    r.color = n.color;
    n.color = RED;
    r.left = Some(n);
    r
}
fn rotate_right(mut n: Box<Node>) -> Box<Node> {
    let mut l = n.left.take().unwrap();
    n.left = l.right.take();
    l.color = n.color;
    n.color = RED;
    l.right = Some(n);
    l
}
fn flip_colors(n: &mut Box<Node>) {
    n.color = !n.color;
    if let Some(l) = n.left.as_mut()  { l.color = !l.color; }
    if let Some(r) = n.right.as_mut() { r.color = !r.color; }
}

fn insert_rec(n: Option<Box<Node>>, k: i32) -> Box<Node> {
    let mut n = match n {
        None => return Box::new(Node { key: k, color: RED, left: None, right: None }),
        Some(b) => b,
    };

    if k < n.key      { n.left  = Some(insert_rec(n.left.take(),  k)); }
    else if k > n.key { n.right = Some(insert_rec(n.right.take(), k)); }
    else { return n; }

    if is_red(&n.right) && !is_red(&n.left) { n = rotate_left(n); }
    if is_red(&n.left) && n.left.as_ref().map_or(false, |l| is_red(&l.left)) {
        n = rotate_right(n);
    }
    if is_red(&n.left) && is_red(&n.right) { flip_colors(&mut n); }
    n
}

pub fn rb_insert(root: Option<Box<Node>>, k: i32) -> Option<Box<Node>> {
    let mut r = insert_rec(root, k);
    r.color = BLACK;
    Some(r)
}

pub fn verify(n: &Option<Box<Node>>) -> (i32, i32) {
    match n {
        None => (0, 0),
        Some(b) => {
            if b.color == RED && (is_red(&b.left) || is_red(&b.right)) {
                panic!("red-red violation at key={}", b.key);
            }
            let (lh, lbh) = verify(&b.left);
            let (rh, rbh) = verify(&b.right);
            assert_eq!(lbh, rbh, "BH mismatch at key={}", b.key);
            (1 + lh.max(rh), lbh + if b.color == BLACK { 1 } else { 0 })
        }
    }
}

fn main() {
    let mut t: Option<Box<Node>> = None;
    for i in 1..=1000 { t = rb_insert(t, i); }
    let (h, bh) = verify(&t);
    println!("sequential 1..1000: height={h} black-height={bh}");

    let mut t: Option<Box<Node>> = None;
    let mut s: u64 = 12345;
    for _ in 0..10_000 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        t = rb_insert(t, ((s >> 32) % 100_000) as i32);
    }
    let (h, bh) = verify(&t);
    println!("random insert n=10000: height={h} black-height={bh}");
}
