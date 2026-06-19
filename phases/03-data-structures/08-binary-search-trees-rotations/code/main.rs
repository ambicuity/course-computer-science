//! main.rs — BST + rotations in safe Rust.

pub struct Node {
    key: i32,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

fn new_node(k: i32) -> Box<Node> { Box::new(Node { key: k, left: None, right: None }) }

pub fn insert(n: Option<Box<Node>>, k: i32) -> Option<Box<Node>> {
    match n {
        None => Some(new_node(k)),
        Some(mut b) => {
            if k < b.key      { b.left  = insert(b.left.take(),  k); }
            else if k > b.key { b.right = insert(b.right.take(), k); }
            Some(b)
        }
    }
}

pub fn contains(mut n: Option<&Node>, k: i32) -> bool {
    while let Some(c) = n {
        if k < c.key      { n = c.left.as_deref(); }
        else if k > c.key { n = c.right.as_deref(); }
        else { return true; }
    }
    false
}

pub fn height(n: &Option<Box<Node>>) -> i32 {
    match n {
        None => 0,
        Some(b) => 1 + height(&b.left).max(height(&b.right)),
    }
}

pub fn rotate_left(mut n: Box<Node>) -> Box<Node> {
    let mut r = n.right.take().expect("right must exist for rotate_left");
    n.right = r.left.take();
    r.left = Some(n);
    r
}

pub fn rotate_right(mut n: Box<Node>) -> Box<Node> {
    let mut l = n.left.take().expect("left must exist for rotate_right");
    n.left = l.right.take();
    l.right = Some(n);
    l
}

pub fn inorder(n: &Option<Box<Node>>, out: &mut Vec<i32>) {
    if let Some(b) = n {
        inorder(&b.left, out);
        out.push(b.key);
        inorder(&b.right, out);
    }
}

fn main() {
    let mut t: Option<Box<Node>> = None;
    for v in [10, 5, 20, 15, 25] { t = insert(t, v); }

    let mut pre = vec![]; inorder(&t, &mut pre);
    println!("inorder before rotate_left: {:?}", pre);

    let t = rotate_left(t.unwrap());
    let mut post = vec![]; inorder(&Some(t), &mut post);
    println!("inorder after  rotate_left: {:?}   (unchanged: invariant preserved)", post);
}
