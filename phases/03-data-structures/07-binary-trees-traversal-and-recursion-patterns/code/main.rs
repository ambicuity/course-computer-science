//! main.rs — binary-tree traversals in Rust (Option<Box<Node>>).

use std::collections::VecDeque;

pub struct Node {
    data: i32,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

fn leaf(v: i32) -> Box<Node> { Box::new(Node { data: v, left: None, right: None }) }
fn node(v: i32, l: Option<Box<Node>>, r: Option<Box<Node>>) -> Box<Node> {
    Box::new(Node { data: v, left: l, right: r })
}

fn preorder(n: &Option<Box<Node>>, out: &mut Vec<i32>) {
    if let Some(n) = n { out.push(n.data); preorder(&n.left, out); preorder(&n.right, out); }
}
fn inorder(n: &Option<Box<Node>>, out: &mut Vec<i32>) {
    if let Some(n) = n { inorder(&n.left, out); out.push(n.data); inorder(&n.right, out); }
}
fn postorder(n: &Option<Box<Node>>, out: &mut Vec<i32>) {
    if let Some(n) = n { postorder(&n.left, out); postorder(&n.right, out); out.push(n.data); }
}

fn inorder_iter(root: &Option<Box<Node>>) -> Vec<i32> {
    let mut out = vec![];
    let mut stack: Vec<&Node> = vec![];
    let mut cur = root.as_deref();
    while cur.is_some() || !stack.is_empty() {
        while let Some(c) = cur { stack.push(c); cur = c.left.as_deref(); }
        let c = stack.pop().unwrap();
        out.push(c.data);
        cur = c.right.as_deref();
    }
    out
}

fn bfs(root: &Option<Box<Node>>) -> Vec<i32> {
    let mut out = vec![];
    let mut q: VecDeque<&Node> = VecDeque::new();
    if let Some(r) = root.as_deref() { q.push_back(r); }
    while let Some(n) = q.pop_front() {
        out.push(n.data);
        if let Some(l) = n.left.as_deref()  { q.push_back(l); }
        if let Some(r) = n.right.as_deref() { q.push_back(r); }
    }
    out
}

fn stats(n: &Option<Box<Node>>) -> (i32, i32, bool) {
    let n = match n { Some(n) => n, None => return (0, 0, true) };
    let (lh, ld, lb) = stats(&n.left);
    let (rh, rd, rb) = stats(&n.right);
    let h = 1 + lh.max(rh);
    let d = ld.max(rd).max(lh + rh);
    let bal = lb && rb && (lh - rh).abs() <= 1;
    (h, d, bal)
}

fn main() {
    let t: Option<Box<Node>> = Some(node(1,
        Some(node(2, Some(leaf(4)), Some(node(5, None, Some(leaf(7)))))),
        Some(node(3, None, Some(leaf(6))))));

    let mut p = vec![]; preorder(&t, &mut p); println!("preorder : {:?}", p);
    let mut i = vec![]; inorder(&t, &mut i);  println!("inorder  : {:?}", i);
    let mut o = vec![]; postorder(&t, &mut o); println!("postorder: {:?}", o);
    println!("inorder_iter: {:?}", inorder_iter(&t));
    println!("BFS         : {:?}", bfs(&t));

    let (h, d, bal) = stats(&t);
    println!("\nheight={} diameter={} balanced={}", h, d, bal);
}
