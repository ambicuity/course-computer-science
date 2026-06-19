//! main.rs — iterative segment tree + Fenwick in Rust.

pub struct SegTree { t: Vec<i64>, n: usize }

impl SegTree {
    pub fn new(a: &[i64]) -> Self {
        let n = a.len();
        let mut t = vec![0i64; 2 * n];
        for (i, &x) in a.iter().enumerate() { t[i + n] = x; }
        for i in (1..n).rev() { t[i] = t[2*i] + t[2*i+1]; }
        SegTree { t, n }
    }
    pub fn update(&mut self, mut i: usize, x: i64) {
        i += self.n;
        self.t[i] = x;
        while i > 1 { i >>= 1; self.t[i] = self.t[2*i] + self.t[2*i+1]; }
    }
    pub fn query(&self, mut l: usize, mut r: usize) -> i64 {
        let mut res = 0i64;
        l += self.n; r += self.n;
        while l < r {
            if l & 1 == 1 { res += self.t[l]; l += 1; }
            if r & 1 == 1 { r -= 1; res += self.t[r]; }
            l >>= 1; r >>= 1;
        }
        res
    }
}

pub struct Fenwick { b: Vec<i64>, n: usize }

impl Fenwick {
    pub fn new(n: usize) -> Self { Fenwick { b: vec![0i64; n + 1], n } }
    pub fn add(&mut self, mut i: usize, x: i64) {
        i += 1;
        while i <= self.n { self.b[i] += x; i += i & i.wrapping_neg(); }
    }
    pub fn prefix(&self, mut i: usize) -> i64 {
        let mut s = 0;
        while i > 0 { s += self.b[i]; i -= i & i.wrapping_neg(); }
        s
    }
    pub fn range(&self, l: usize, r: usize) -> i64 { self.prefix(r) - self.prefix(l) }
}

fn main() {
    let a = vec![1i64, 3, 5, 7, 9, 11, 13, 15];
    let mut st = SegTree::new(&a);
    println!("SegTree.query(0, 8) = {}  (expect 64)", st.query(0, 8));
    println!("SegTree.query(2, 5) = {}  (expect 21)", st.query(2, 5));
    st.update(2, 100);
    println!("after update(2, 100): query(2, 5) = {}  (expect 116)", st.query(2, 5));

    let mut f = Fenwick::new(a.len());
    for (i, &x) in a.iter().enumerate() { f.add(i, x); }
    println!("\nFenwick.range(0, 8) = {}  (expect 64)", f.range(0, 8));
    println!("Fenwick.range(2, 5) = {}  (expect 21)", f.range(2, 5));
    f.add(2, 95);
    println!("after add(2, +95): range(2, 5) = {}  (expect 116)", f.range(2, 5));
}
