//! main.rs — DSU + Kruskal in Rust.

pub struct Dsu { parent: Vec<usize>, rank_: Vec<u8> }

impl Dsu {
    pub fn new(n: usize) -> Self {
        Dsu { parent: (0..n).collect(), rank_: vec![0; n] }
    }

    pub fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];   // path halving
            x = self.parent[x];
        }
        x
    }

    pub fn unite(&mut self, x: usize, y: usize) -> bool {
        let (rx, ry) = (self.find(x), self.find(y));
        if rx == ry { return false; }
        match self.rank_[rx].cmp(&self.rank_[ry]) {
            std::cmp::Ordering::Less => self.parent[rx] = ry,
            std::cmp::Ordering::Greater => self.parent[ry] = rx,
            std::cmp::Ordering::Equal => {
                self.parent[ry] = rx;
                self.rank_[rx] += 1;
            }
        }
        true
    }

    pub fn connected(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }
}

fn kruskal(n: usize, mut edges: Vec<(usize, usize, i32)>) -> i32 {
    edges.sort_by_key(|e| e.2);
    let mut d = Dsu::new(n);
    let mut total = 0i32;
    let mut picked = 0;
    for (u, v, w) in edges {
        if d.unite(u, v) {
            total += w;
            picked += 1;
            if picked == n - 1 { break; }
        }
    }
    if picked == n - 1 { total } else { -1 }
}

fn main() {
    let mut d = Dsu::new(10);
    for (u, v) in [(1, 2), (2, 3), (5, 6), (7, 6)] { d.unite(u, v); }
    println!("connected(1,3) = {}", d.connected(1, 3));
    println!("connected(1,5) = {}", d.connected(1, 5));

    let edges = vec![
        (0, 1, 4), (0, 2, 3), (1, 2, 1), (1, 3, 2),
        (2, 3, 4), (3, 4, 2), (4, 0, 4), (4, 2, 4),
    ];
    println!("Kruskal MST: {}  (expect 8)", kruskal(5, edges));
}
