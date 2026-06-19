//! main.rs — dynamic array in Rust with cost accounting + std::Vec comparison.
//!
//! Build: `rustc -O main.rs && ./main`

struct MyVec<T> {
    data: Vec<Option<T>>,
    len: usize,
    cap: usize,
    resizes: usize,
    total_copies: usize,
}

impl<T: Clone + Default> MyVec<T> {
    fn new() -> Self {
        let cap = 4;
        MyVec {
            data: (0..cap).map(|_| None).collect(),
            len: 0,
            cap,
            resizes: 0,
            total_copies: 0,
        }
    }

    fn push_factor(&mut self, x: T, factor: f64) {
        if self.len == self.cap {
            let new_cap = ((self.cap as f64) * factor) as usize;
            let new_cap = new_cap.max(self.cap + 1);
            self.total_copies += self.len;
            self.resizes += 1;
            let mut new_data: Vec<Option<T>> = (0..new_cap).map(|_| None).collect();
            for i in 0..self.len {
                new_data[i] = self.data[i].take();
            }
            self.data = new_data;
            self.cap = new_cap;
        }
        self.data[self.len] = Some(x);
        self.len += 1;
    }
}

fn report(label: &str, v: &MyVec<i32>, n: usize) {
    let amortized = (n + v.total_copies) as f64 / n as f64;
    println!(
        "{label:>14}: cap={:>8}  resizes={:>6}  copies={:>10}  amortized={amortized:.2} writes/push",
        v.cap, v.resizes, v.total_copies
    );
}

fn main() {
    let n: usize = 200_000;
    println!("== MyVec<i32> growth: {} pushes ==\n", n);

    let mut v2: MyVec<i32> = MyVec::new();
    for i in 0..n { v2.push_factor(i as i32, 2.0); }
    report("2.0× growth", &v2, n);

    let mut v15: MyVec<i32> = MyVec::new();
    for i in 0..n { v15.push_factor(i as i32, 1.5); }
    report("1.5× growth", &v15, n);

    println!();
    println!("== Rust's Vec::push uses 2× growth, see raw_vec.rs::grow_amortized ==");
    let t0 = std::time::Instant::now();
    let mut sv: Vec<i32> = Vec::new();
    for i in 0..n { sv.push(i as i32); }
    let t = t0.elapsed();
    println!("  Vec::push × {n}: {:.1?}  ({} ns/op)", t, t.as_nanos() as usize / n);

    let t0 = std::time::Instant::now();
    let mut sv2: Vec<i32> = Vec::with_capacity(n);
    for i in 0..n { sv2.push(i as i32); }
    let t = t0.elapsed();
    println!("  Vec::with_capacity then push × {n}: {:.1?}  ({} ns/op)", t, t.as_nanos() as usize / n);
}
