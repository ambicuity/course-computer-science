//! main.rs — sparse table RMQ in Rust.

pub struct RMQ {
    t: Vec<Vec<i32>>,
    log2: Vec<usize>,
}

impl RMQ {
    pub fn new(a: &[i32]) -> Self {
        let n = a.len();
        let mut log2 = vec![0usize; n + 1];
        for i in 2..=n { log2[i] = log2[i / 2] + 1; }
        let max_k = log2[n] + 1;
        let mut t: Vec<Vec<i32>> = vec![a.to_vec()];
        for k in 1..max_k {
            let half = 1 << (k - 1);
            let len = if n + 1 > (1 << k) { n + 1 - (1 << k) } else { 0 };
            let prev = &t[k - 1];
            let row: Vec<i32> = (0..len).map(|i| prev[i].min(prev[i + half])).collect();
            t.push(row);
        }
        RMQ { t, log2 }
    }

    pub fn query(&self, l: usize, r: usize) -> i32 {        // inclusive
        let k = self.log2[r - l + 1];
        self.t[k][l].min(self.t[k][r + 1 - (1 << k)])
    }
}

fn main() {
    let a: Vec<i32> = (0..500).map(|i| ((i * 1103515245 + 12345) % 1000) as i32).collect();
    let rmq = RMQ::new(&a);
    let mut ok = true;
    for t in 0..200 {
        let l = (t * 13) % 500;
        let r = (l + (t * 7) % (500 - l)).min(499);
        let naive: i32 = a[l..=r].iter().copied().min().unwrap();
        if rmq.query(l, r) != naive { ok = false; break; }
    }
    println!("RMQ verified on 200 deterministic queries: {ok}");
    println!("query(10, 50) = {}", rmq.query(10, 50));
}
