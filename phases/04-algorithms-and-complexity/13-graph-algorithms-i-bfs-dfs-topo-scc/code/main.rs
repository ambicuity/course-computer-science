//! Graph Algorithms I — BFS, DFS, Topo, SCC
//! Phase 04 — Algorithms & Complexity Analysis
//!
//! Adjacency-list graph with BFS, DFS, Kahn's topo sort, Tarjan SCC, Kosaraju SCC.

use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

struct Graph {
    n: usize,
    directed: bool,
    adj: Vec<Vec<usize>>,
}

impl Graph {
    fn new(n: usize, directed: bool) -> Self {
        Self {
            n,
            directed,
            adj: vec![vec![]; n],
        }
    }

    fn add_edge(&mut self, u: usize, v: usize) {
        self.adj[u].push(v);
        if !self.directed {
            self.adj[v].push(u);
        }
    }

    fn reverse(&self) -> Graph {
        let mut gt = Graph::new(self.n, true);
        for u in 0..self.n {
            for &v in &self.adj[u] {
                gt.adj[v].push(u);
            }
        }
        gt
    }
}

// ---------------------------------------------------------------------------
// BFS — parent map + distances
// ---------------------------------------------------------------------------

fn bfs(g: &Graph, start: usize) -> (Vec<i32>, Vec<i32>) {
    let mut dist = vec![-1i32; g.n];
    let mut parent = vec![-1i32; g.n];
    let mut q = VecDeque::new();
    dist[start] = 0;
    q.push_back(start);
    while let Some(u) = q.pop_front() {
        for &v in &g.adj[u] {
            if dist[v] == -1 {
                dist[v] = dist[u] + 1;
                parent[v] = u as i32;
                q.push_back(v);
            }
        }
    }
    (parent, dist)
}

// ---------------------------------------------------------------------------
// DFS — discovery / finish times (iterative)
// ---------------------------------------------------------------------------

fn dfs(g: &Graph, start: usize) -> (Vec<i32>, Vec<i32>, Vec<i32>) {
    const WHITE: u8 = 0;
    const GRAY: u8 = 1;
    const BLACK: u8 = 2;
    let mut color = vec![WHITE; g.n];
    let mut parent = vec![-1i32; g.n];
    let mut disc = vec![-1i32; g.n];
    let mut fin = vec![-1i32; g.n];
    let mut time: i32 = 0;

    // Stack: (vertex, finished_flag)
    let mut stack: Vec<(usize, bool)> = vec![(start, false)];
    color[start] = GRAY;
    disc[start] = time;
    time += 1;

    while let Some((u, finished)) = stack.pop() {
        if finished {
            fin[u] = time;
            time += 1;
            color[u] = BLACK;
            continue;
        }
        stack.push((u, true));
        for &v in &g.adj[u] {
            if color[v] == WHITE {
                color[v] = GRAY;
                parent[v] = u as i32;
                disc[v] = time;
                time += 1;
                stack.push((v, false));
            }
        }
    }
    (parent, disc, fin)
}

// ---------------------------------------------------------------------------
// Bipartite check — BFS 2-coloring
// ---------------------------------------------------------------------------

fn is_bipartite(g: &Graph) -> (bool, Option<Vec<i32>>) {
    let mut color = vec![-1i32; g.n];
    for s in 0..g.n {
        if color[s] != -1 {
            continue;
        }
        color[s] = 0;
        let mut q = VecDeque::new();
        q.push_back(s);
        while let Some(u) = q.pop_front() {
            for &v in &g.adj[u] {
                if color[v] == -1 {
                    color[v] = 1 - color[u];
                    q.push_back(v);
                } else if color[v] == color[u] {
                    return (false, None);
                }
            }
        }
    }
    (true, Some(color))
}

// ---------------------------------------------------------------------------
// Cycle detection (directed) — DFS-based
// ---------------------------------------------------------------------------

fn has_cycle_directed(g: &Graph) -> bool {
    const WHITE: u8 = 0;
    const GRAY: u8 = 1;
    const BLACK: u8 = 2;
    let mut color = vec![WHITE; g.n];

    fn visit(g: &Graph, u: usize, color: &mut [u8]) -> bool {
        color[u] = GRAY;
        for &v in &g.adj[u] {
            if color[v] == GRAY {
                return true;
            }
            if color[v] == WHITE && visit(g, v, color) {
                return true;
            }
        }
        color[u] = BLACK;
        false
    }

    for u in 0..g.n {
        if color[u] == WHITE && visit(g, u, &mut color) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Topological sort — Kahn's (BFS)
// ---------------------------------------------------------------------------

fn topological_sort_kahn(g: &Graph) -> Result<Vec<usize>, &'static str> {
    let mut in_deg = vec![0usize; g.n];
    for u in 0..g.n {
        for &v in &g.adj[u] {
            in_deg[v] += 1;
        }
    }
    let mut q: VecDeque<usize> = (0..g.n).filter(|&u| in_deg[u] == 0).collect();
    let mut order = Vec::with_capacity(g.n);
    while let Some(u) = q.pop_front() {
        order.push(u);
        for &v in &g.adj[u] {
            in_deg[v] -= 1;
            if in_deg[v] == 0 {
                q.push_back(v);
            }
        }
    }
    if order.len() != g.n {
        Err("graph has a cycle")
    } else {
        Ok(order)
    }
}

// ---------------------------------------------------------------------------
// Topological sort — DFS reverse-postorder
// ---------------------------------------------------------------------------

fn topological_sort_dfs(g: &Graph) -> Result<Vec<usize>, &'static str> {
    const WHITE: u8 = 0;
    const GRAY: u8 = 1;
    const BLACK: u8 = 2;
    let mut color = vec![WHITE; g.n];
    let mut order = Vec::with_capacity(g.n);

    fn visit(g: &Graph, u: usize, color: &mut [u8], order: &mut Vec<usize>) -> Result<(), &'static str> {
        color[u] = GRAY;
        for &v in &g.adj[u] {
            if color[v] == GRAY {
                return Err("graph has a cycle");
            }
            if color[v] == WHITE {
                visit(g, v, color, order)?;
            }
        }
        color[u] = BLACK;
        order.push(u);
        Ok(())
    }

    for u in 0..g.n {
        if color[u] == WHITE {
            visit(g, u, &mut color, &mut order)?;
        }
    }
    order.reverse();
    Ok(order)
}

// ---------------------------------------------------------------------------
// Tarjan's SCC
// ---------------------------------------------------------------------------

fn tarjan_scc(g: &Graph) -> Vec<Vec<usize>> {
    let mut index = vec![-1i32; g.n];
    let mut low = vec![0i32; g.n];
    let mut on_stack = vec![false; g.n];
    let mut stack: Vec<usize> = Vec::new();
    let mut time: i32 = 0;
    let mut sccs: Vec<Vec<usize>> = Vec::new();

    fn strongconnect(
        g: &Graph,
        v: usize,
        index: &mut [i32],
        low: &mut [i32],
        on_stack: &mut [bool],
        stack: &mut Vec<usize>,
        time: &mut i32,
        sccs: &mut Vec<Vec<usize>>,
    ) {
        index[v] = *time;
        low[v] = *time;
        *time += 1;
        stack.push(v);
        on_stack[v] = true;

        for &w in &g.adj[v] {
            if index[w] == -1 {
                strongconnect(g, w, index, low, on_stack, stack, time, sccs);
                low[v] = low[v].min(low[w]);
            } else if on_stack[w] {
                low[v] = low[v].min(index[w]);
            }
        }

        if low[v] == index[v] {
            let mut scc = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack[w] = false;
                scc.push(w);
                if w == v {
                    break;
                }
            }
            sccs.push(scc);
        }
    }

    for v in 0..g.n {
        if index[v] == -1 {
            strongconnect(g, v, &mut index, &mut low, &mut on_stack, &mut stack, &mut time, &mut sccs);
        }
    }
    sccs
}

// ---------------------------------------------------------------------------
// Kosaraju's SCC
// ---------------------------------------------------------------------------

fn kosaraju_scc(g: &Graph) -> Vec<Vec<usize>> {
    let mut visited = vec![false; g.n];
    let mut finish_order: Vec<usize> = Vec::new();

    fn dfs1(g: &Graph, u: usize, visited: &mut [bool], order: &mut Vec<usize>) {
        visited[u] = true;
        for &v in &g.adj[u] {
            if !visited[v] {
                dfs1(g, v, visited, order);
            }
        }
        order.push(u);
    }

    for u in 0..g.n {
        if !visited[u] {
            dfs1(g, u, &mut visited, &mut finish_order);
        }
    }

    let gt = g.reverse();
    visited.fill(false);
    let mut sccs: Vec<Vec<usize>> = Vec::new();

    for &u in finish_order.iter().rev() {
        if !visited[u] {
            let mut scc = Vec::new();
            let mut stack = vec![u];
            visited[u] = true;
            while let Some(v) = stack.pop() {
                scc.push(v);
                for &w in &gt.adj[v] {
                    if !visited[w] {
                        visited[w] = true;
                        stack.push(w);
                    }
                }
            }
            sccs.push(scc);
        }
    }
    sccs
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("=== Graph Algorithms I — BFS, DFS, Topo, SCC ===\n");

    // --- BFS ---
    println!("--- BFS from node 0 ---");
    let mut ug = Graph::new(7, false);
    for (u, v) in [(0, 1), (0, 2), (1, 3), (1, 4), (2, 5), (2, 6)] {
        ug.add_edge(u, v);
    }
    let (parent, dist) = bfs(&ug, 0);
    println!("  Distances: {:?}", dist);
    println!("  Parents:   {:?}", parent);
    let mut path = vec![];
    let mut v: i32 = 4;
    while v != -1 {
        path.push(v);
        v = parent[v as usize];
    }
    path.reverse();
    println!("  Shortest path 0->4: {:?}", path);
    println!();

    // --- DFS ---
    println!("--- DFS from node 0 ---");
    let mut dg = Graph::new(6, true);
    for (u, v) in [(0, 1), (0, 2), (1, 3), (2, 3), (3, 4), (4, 5)] {
        dg.add_edge(u, v);
    }
    let (_, disc, fin) = dfs(&dg, 0);
    println!("  Discovery: {:?}", disc);
    println!("  Finish:    {:?}", fin);
    println!();

    // --- Bipartite check ---
    println!("--- Bipartite check ---");
    let (ok, colors) = is_bipartite(&ug);
    println!("  Tree (bipartite): {ok}, colors={colors:?}");
    let mut odd_cycle = Graph::new(3, false);
    odd_cycle.add_edge(0, 1);
    odd_cycle.add_edge(1, 2);
    odd_cycle.add_edge(2, 0);
    let (ok2, _) = is_bipartite(&odd_cycle);
    println!("  Triangle (not bipartite): {ok2}");
    println!();

    // --- Cycle detection ---
    println!("--- Cycle detection (directed) ---");
    let mut dag = Graph::new(4, true);
    dag.add_edge(0, 1);
    dag.add_edge(1, 2);
    dag.add_edge(2, 3);
    println!("  DAG has cycle: {}", has_cycle_directed(&dag));
    let mut cyclic = Graph::new(4, true);
    cyclic.add_edge(0, 1);
    cyclic.add_edge(1, 2);
    cyclic.add_edge(2, 0);
    cyclic.add_edge(2, 3);
    println!("  Cyclic graph has cycle: {}", has_cycle_directed(&cyclic));
    println!();

    // --- Topological sort ---
    println!("--- Topological sort ---");
    let mut ts = Graph::new(6, true);
    for (u, v) in [(5, 2), (5, 0), (4, 0), (4, 1), (2, 3), (3, 1)] {
        ts.add_edge(u, v);
    }
    let order_kahn = topological_sort_kahn(&ts).unwrap();
    let order_dfs = topological_sort_dfs(&ts).unwrap();
    println!("  Kahn's:     {:?}", order_kahn);
    println!("  DFS-based:  {:?}", order_dfs);
    println!();

    // --- SCC ---
    println!("--- Strongly Connected Components ---");
    let mut sg = Graph::new(8, true);
    for (u, v) in [(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 5), (5, 3), (6, 5), (6, 7), (7, 6)] {
        sg.add_edge(u, v);
    }
    let tarjan_result = tarjan_scc(&sg);
    let kosaraju_result = kosaraju_scc(&sg);
    let mut tarjan_sorted: Vec<Vec<usize>> = tarjan_result.into_iter().map(|mut s| { s.sort(); s }).collect();
    let mut kosaraju_sorted: Vec<Vec<usize>> = kosaraju_result.into_iter().map(|mut s| { s.sort(); s }).collect();
    tarjan_sorted.sort();
    kosaraju_sorted.sort();
    println!("  Tarjan:   {:?}", tarjan_sorted);
    println!("  Kosaraju: {:?}", kosaraju_sorted);
    println!();

    // --- Module dependency build order ---
    println!("--- Build order (topo sort on module deps) ---");
    let names = ["core", "api", "utils", "db", "cache"];
    let mut modules = Graph::new(5, true);
    modules.add_edge(0, 2);
    modules.add_edge(1, 2);
    modules.add_edge(2, 3);
    modules.add_edge(2, 4);
    modules.add_edge(3, 4);
    let order = topological_sort_kahn(&modules).unwrap();
    let named: Vec<&str> = order.iter().map(|&i| names[i]).collect();
    println!("  Build order: {:?}", named);
}
