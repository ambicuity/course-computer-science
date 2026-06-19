use std::collections::HashMap;
use std::env;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Sorting Algorithms
// ---------------------------------------------------------------------------

fn insertion_sort(arr: &mut [i32]) {
    for i in 1..arr.len() {
        let key = arr[i];
        let mut j = i;
        while j > 0 && arr[j - 1] > key {
            arr[j] = arr[j - 1];
            j -= 1;
        }
        arr[j] = key;
    }
}

fn selection_sort(arr: &mut [i32]) {
    let n = arr.len();
    for i in 0..n {
        let mut min_idx = i;
        for j in (i + 1)..n {
            if arr[j] < arr[min_idx] {
                min_idx = j;
            }
        }
        arr.swap(i, min_idx);
    }
}

fn merge_sort(arr: &mut [i32]) {
    let n = arr.len();
    if n <= 1 {
        return;
    }
    let mid = n / 2;
    merge_sort(&mut arr[..mid]);
    merge_sort(&mut arr[mid..]);
    let mut buf = arr.to_vec();
    let (left, right) = buf.split_at(mid);
    let (mut i, mut j, mut k) = (0, 0, 0);
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            arr[k] = left[i];
            i += 1;
        } else {
            arr[k] = right[j];
            j += 1;
        }
        k += 1;
    }
    while i < left.len() {
        arr[k] = left[i];
        i += 1;
        k += 1;
    }
    while j < right.len() {
        arr[k] = right[j];
        j += 1;
        k += 1;
    }
}

fn quick_sort(arr: &mut [i32]) {
    qs(arr, 0, arr.len() as isize - 1);
}

fn qs(arr: &mut [i32], lo: isize, hi: isize) {
    if lo >= hi {
        return;
    }
    let mid = (lo + hi) / 2;
    let mut pivots = [(arr[lo as usize], lo), (arr[mid as usize], mid), (arr[hi as usize], hi)];
    pivots.sort_by_key(|&(v, _)| v);
    let piv_idx = pivots[1].1 as usize;
    arr.swap(lo as usize, piv_idx);
    let pivot = arr[lo as usize];
    let mut i = lo + 1;
    for j in (lo + 1)..=hi {
        if arr[j as usize] < pivot {
            arr.swap(i as usize, j as usize);
            i += 1;
        }
    }
    arr.swap(lo as usize, (i - 1) as usize);
    qs(arr, lo, i - 2);
    qs(arr, i, hi);
}

fn heap_sort(arr: &mut [i32]) {
    let n = arr.len();
    for i in (0..n / 2).rev() {
        sift_down(arr, i, n);
    }
    for end in (1..n).rev() {
        arr.swap(0, end);
        sift_down(arr, 0, end);
    }
}

fn sift_down(arr: &mut [i32], mut root: usize, size: usize) {
    loop {
        let left = 2 * root + 1;
        let right = 2 * root + 2;
        let mut largest = root;
        if left < size && arr[left] > arr[largest] {
            largest = left;
        }
        if right < size && arr[right] > arr[largest] {
            largest = right;
        }
        if largest == root {
            break;
        }
        arr.swap(root, largest);
        root = largest;
    }
}

// ---------------------------------------------------------------------------
// Searching Algorithms
// ---------------------------------------------------------------------------

fn binary_search(arr: &[i32], target: i32) -> Option<usize> {
    let (mut lo, mut hi) = (0isize, arr.len() as isize - 1);
    while lo <= hi {
        let mid = (lo + hi) / 2;
        if arr[mid as usize] == target {
            return Some(mid as usize);
        } else if arr[mid as usize] < target {
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    None
}

fn linear_search(arr: &[i32], target: i32) -> Option<usize> {
    for (i, &v) in arr.iter().enumerate() {
        if v == target {
            return Some(i);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Graph Algorithms
// ---------------------------------------------------------------------------

fn bfs(graph: &HashMap<usize, Vec<usize>>, src: usize) -> HashMap<usize, usize> {
    let mut dist = HashMap::new();
    let mut queue = vec![src];
    dist.insert(src, 0);
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        if let Some(neighbors) = graph.get(&u) {
            for &v in neighbors {
                if !dist.contains_key(&v) {
                    dist.insert(v, dist[&u] + 1);
                    queue.push(v);
                }
            }
        }
    }
    dist
}

fn dfs(graph: &HashMap<usize, Vec<usize>>, src: usize) -> Vec<usize> {
    let mut visited = Vec::new();
    let mut stack = vec![src];
    let mut seen = std::collections::HashSet::new();
    while let Some(u) = stack.pop() {
        if seen.contains(&u) {
            continue;
        }
        seen.insert(u);
        visited.push(u);
        if let Some(neighbors) = graph.get(&u) {
            for &v in neighbors.iter().rev() {
                if !seen.contains(&v) {
                    stack.push(v);
                }
            }
        }
    }
    visited
}

// ---------------------------------------------------------------------------
// Input Generators
// ---------------------------------------------------------------------------

fn gen_random(n: usize) -> Vec<i32> {
    (0..n).map(|_| rand_i32(n as i32)).collect()
}

fn gen_sorted(n: usize) -> Vec<i32> {
    (0..n as i32).collect()
}

fn gen_reversed(n: usize) -> Vec<i32> {
    (1..=n as i32).rev().collect()
}

fn gen_nearly_sorted(n: usize) -> Vec<i32> {
    let mut arr: Vec<i32> = (0..n as i32).collect();
    for _ in 0..10 {
        let i = rand_usize(n);
        let j = rand_usize(n);
        arr.swap(i, j);
    }
    arr
}

fn gen_adversarial(n: usize) -> Vec<i32> {
    let mut arr: Vec<i32> = (0..n as i32).collect();
    let mid = n / 2;
    arr.swap(0, mid);
    arr
}

// Simple deterministic pseudo-random (no external crate dependency)
static mut RNG_STATE: u64 = 12345;

fn rand_u64() -> u64 {
    unsafe {
        RNG_STATE = RNG_STATE.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        RNG_STATE
    }
}

fn rand_i32(max: i32) -> i32 {
    (rand_u64() as i32).rem_euclid(max)
}

fn rand_usize(max: usize) -> usize {
    (rand_u64() as usize) % max.max(1)
}

fn gen_graph(V: usize, E: usize) -> HashMap<usize, Vec<usize>> {
    let mut graph: HashMap<usize, Vec<usize>> = HashMap::new();
    for _ in 0..E {
        let u = rand_usize(V);
        let v = rand_usize(V);
        graph.entry(u).or_default().push(v);
    }
    for i in 0..V {
        graph.entry(i).or_default();
    }
    graph
}

// ---------------------------------------------------------------------------
// Bench Result + Table Printer
// ---------------------------------------------------------------------------

struct BenchResult {
    algorithm: String,
    input: String,
    n: usize,
    time_us: u128,
}

fn print_sort_table(results: &[BenchResult]) {
    println!("{:<14} {:<14} {:>8} {:>12}", "Algorithm", "Input", "N", "Time (µs)");
    println!("{}", "-".repeat(52));
    for r in results {
        println!("{:<14} {:<14} {:>8} {:>12}", r.algorithm, r.input, r.n, r.time_us);
    }
}

fn print_search_table(results: &[BenchResult]) {
    println!("{:<14} {:<14} {:>8} {:>12}", "Algorithm", "Input", "N", "Time (µs)");
    println!("{}", "-".repeat(52));
    for r in results {
        println!("{:<14} {:<14} {:>8} {:>12}", r.algorithm, r.input, r.n, r.time_us);
    }
}

fn print_graph_table(results: &[BenchResult]) {
    println!("{:<14} {:<20} {:>12}", "Algorithm", "Graph", "Time (µs)");
    println!("{}", "-".repeat(50));
    for r in results {
        println!("{:<14} {:<20} {:>12}", r.algorithm, r.input, r.time_us);
    }
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

fn cmd_sort() {
    println!("=== SORTING BENCHMARKS ===\n");
    let n_values = [500, 2000, 5000];
    let generators: Vec<(&str, fn(usize) -> Vec<i32>)> = vec![
        ("random", gen_random as fn(usize) -> Vec<i32>),
        ("sorted", gen_sorted),
        ("reversed", gen_reversed),
    ];
    let mut results = Vec::new();

    for &n in &n_values {
        for &(gname, gen) in &generators {
            let base = gen(n);
            let algorithms: Vec<(&str, fn(&mut [i32]))> = vec![
                ("insertion", insertion_sort as fn(&mut [i32])),
                ("selection", selection_sort),
                ("merge", merge_sort),
                ("quick", quick_sort),
                ("heap", heap_sort),
            ];
            for (aname, alg_fn) in algorithms {
                let mut times = Vec::new();
                for _ in 0..3 {
                    let mut arr = base.clone();
                    let start = Instant::now();
                    alg_fn(&mut arr);
                    times.push(start.elapsed().as_micros());
                }
                let avg = times.iter().sum::<u128>() / times.len() as u128;
                results.push(BenchResult {
                    algorithm: aname.to_string(),
                    input: gname.to_string(),
                    n,
                    time_us: avg,
                });
            }
        }
    }
    print_sort_table(&results);
}

fn cmd_search() {
    println!("=== SEARCHING BENCHMARKS ===\n");
    let n_values = [10_000, 100_000, 1_000_000];
    let mut results = Vec::new();
    for &n in &n_values {
        let arr: Vec<i32> = (0..n as i32).collect();
        let target = n as i32 / 2;

        // Binary search
        let mut times = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            let _ = binary_search(&arr, target);
            times.push(start.elapsed().as_micros());
        }
        let avg = times.iter().sum::<u128>() / times.len() as u128;
        results.push(BenchResult { algorithm: "binary".into(), input: "sorted".into(), n, time_us: avg });

        // Linear search
        let mut times = Vec::new();
        for _ in 0..3 {
            let start = Instant::now();
            let _ = linear_search(&arr, target);
            times.push(start.elapsed().as_micros());
        }
        let avg = times.iter().sum::<u128>() / times.len() as u128;
        results.push(BenchResult { algorithm: "linear".into(), input: "sorted".into(), n, time_us: avg });
    }
    print_search_table(&results);
}

fn cmd_graph() {
    println!("=== GRAPH BENCHMARKS ===\n");
    let mut results = Vec::new();
    for &V in &[100, 500, 1000] {
        let E = V * 4;
        let g = gen_graph(V, E);

        // BFS
        let mut times = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            let _ = bfs(&g, 0);
            times.push(start.elapsed().as_micros());
        }
        let avg = times.iter().sum::<u128>() / times.len() as u128;
        results.push(BenchResult { algorithm: "BFS".into(), input: format!("V={V} E={E}"), n: V, time_us: avg });

        // DFS
        let mut times = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            let _ = dfs(&g, 0);
            times.push(start.elapsed().as_micros());
        }
        let avg = times.iter().sum::<u128>() / times.len() as u128;
        results.push(BenchResult { algorithm: "DFS".into(), input: format!("V={V} E={E}"), n: V, time_us: avg });
    }
    print_graph_table(&results);
}

fn cmd_report() {
    println!("=== COMBINED BENCHMARK REPORT ===\n");
    cmd_sort();
    println!();
    cmd_search();
    println!();
    cmd_graph();
    println!("\n=== ALGORITHM COOKBOOK QUICK REFERENCE ===\n");
    println!("Sorting:    Bounded range? → Counting/Radix. Nearly sorted? → Insertion.");
    println!("            Need stability? → Merge. General? → Quicksort (median-3).");
    println!("Searching:  Sorted data? → Binary O(log n). Unbounded? → Exponential.");
    println!("            Unsorted? → Linear O(n) or hash table O(1).");
    println!("Graph:      Unweighted? → BFS. Non-negative? → Dijkstra.");
    println!("            Negative? → Bellman-Ford. MST? → Kruskal/Prim.");
    println!("Optimize:   Greedy property? → Greedy. Otherwise → DP.");
    println!("Strings:    Single pattern? → KMP/Boyer-Moore. Multiple? → Aho-Corasick.");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let subcommand = args.get(1).map(|s| s.as_str()).unwrap_or("report");

    match subcommand {
        "sort" => cmd_sort(),
        "search" => cmd_search(),
        "graph" => cmd_graph(),
        "report" => cmd_report(),
        _ => {
            eprintln!("Usage: algorithm_cookbook [sort|search|graph|report]");
            eprintln!("  sort    — benchmark all sorting algorithms");
            eprintln!("  search  — benchmark all searching algorithms");
            eprintln!("  graph   — benchmark graph algorithms");
            eprintln!("  report  — generate combined comparison report");
        }
    }
}
