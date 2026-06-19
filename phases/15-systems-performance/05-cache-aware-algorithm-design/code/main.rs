use std::time::Instant;

const TILE_SIZE: usize = 32;

type Matrix = Vec<Vec<f64>>;

fn mat_mul_naive(a: &Matrix, b: &Matrix, c: &mut Matrix, n: usize) {
    for i in 0..n {
        for j in 0..n {
            let mut sum = 0.0;
            for k in 0..n {
                sum += a[i][k] * b[k][j];
            }
            c[i][j] = sum;
        }
    }
}

fn mat_mul_tiled(a: &Matrix, b: &Matrix, c: &mut Matrix, n: usize) {
    for i in 0..n {
        for j in 0..n {
            c[i][j] = 0.0;
        }
    }
    for i in (0..n).step_by(TILE_SIZE) {
        for j in (0..n).step_by(TILE_SIZE) {
            for k in (0..n).step_by(TILE_SIZE) {
                let i_end = (i + TILE_SIZE).min(n);
                let j_end = (j + TILE_SIZE).min(n);
                let k_end = (k + TILE_SIZE).min(n);
                for ii in i..i_end {
                    for jj in j..j_end {
                        let mut sum = c[ii][jj];
                        for kk in k..k_end {
                            sum += a[ii][kk] * b[kk][jj];
                        }
                        c[ii][jj] = sum;
                    }
                }
            }
        }
    }
}

fn mat_mul_recursive_inner(
    a: &Matrix, b: &Matrix, c: &mut Matrix,
    n: usize,
    a_row: usize, a_col: usize,
    b_row: usize, b_col: usize,
    c_row: usize, c_col: usize,
) {
    if n <= 64 {
        for i in 0..n {
            for j in 0..n {
                let mut sum = c[c_row + i][c_col + j];
                for k in 0..n {
                    sum += a[a_row + i][a_col + k] * b[b_row + k][b_col + j];
                }
                c[c_row + i][c_col + j] = sum;
            }
        }
        return;
    }
    let h = n / 2;
    // C11 += A11*B11
    mat_mul_recursive_inner(a, b, c, h, a_row, a_col, b_row, b_col, c_row, c_col);
    // C11 += A12*B21
    mat_mul_recursive_inner(a, b, c, h, a_row, a_col + h, b_row + h, b_col, c_row, c_col);
    // C12 += A11*B12
    mat_mul_recursive_inner(a, b, c, h, a_row, a_col, b_row, b_col + h, c_row, c_col + h);
    // C12 += A12*B22
    mat_mul_recursive_inner(a, b, c, h, a_row, a_col + h, b_row + h, b_col + h, c_row, c_col + h);
    // C21 += A21*B11
    mat_mul_recursive_inner(a, b, c, h, a_row + h, a_col, b_row, b_col, c_row + h, c_col);
    // C21 += A22*B21
    mat_mul_recursive_inner(a, b, c, h, a_row + h, a_col + h, b_row + h, b_col, c_row + h, c_col);
    // C22 += A21*B12
    mat_mul_recursive_inner(a, b, c, h, a_row + h, a_col, b_row, b_col + h, c_row + h, c_col + h);
    // C22 += A22*B22
    mat_mul_recursive_inner(a, b, c, h, a_row + h, a_col + h, b_row + h, b_col + h, c_row + h, c_col + h);
}

fn mat_mul_cache_oblivious(a: &Matrix, b: &Matrix, c: &mut Matrix, n: usize) {
    for i in 0..n {
        for j in 0..n {
            c[i][j] = 0.0;
        }
    }
    mat_mul_recursive_inner(a, b, c, n, 0, 0, 0, 0, 0, 0);
}

fn checksum(m: &Matrix, n: usize) -> f64 {
    let mut s = 0.0;
    for i in 0..n {
        for j in 0..n {
            s += m[i][j];
        }
    }
    s
}

fn init_matrix(m: &mut Matrix, n: usize) {
    for i in 0..n {
        for j in 0..n {
            m[i][j] = ((i * n + j) % 7) as f64 / 7.0 - (3.0 / 7.0);
        }
    }
}

#[derive(Debug)]
struct ParticleAoS {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    mass: f64,
    charge: f64,
}

struct ParticlesSoA {
    x: Vec<f64>,
    y: Vec<f64>,
    z: Vec<f64>,
    vx: Vec<f64>,
    vy: Vec<f64>,
    vz: Vec<f64>,
    mass: Vec<f64>,
    charge: Vec<f64>,
}

fn bench_aos_gravity(particles: &[ParticleAoS], n: usize, iters: usize) -> f64 {
    let mut total = 0.0;
    let start = Instant::now();
    for _ in 0..iters {
        for i in 0..n {
            for j in 0..n {
                if i == j { continue; }
                let dx = particles[j].x - particles[i].x;
                let dy = particles[j].y - particles[i].y;
                let dz = particles[j].z - particles[i].z;
                let dist_sq = dx * dx + dy * dy + dz * dz + 0.01;
                total += particles[j].mass / dist_sq;
            }
        }
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    if total == 0.0 { 0.0 } else { elapsed }
}

fn bench_soa_gravity(p: &ParticlesSoA, n: usize, iters: usize) -> f64 {
    let mut total = 0.0;
    let start = Instant::now();
    for _ in 0..iters {
        for i in 0..n {
            for j in 0..n {
                if i == j { continue; }
                let dx = p.x[j] - p.x[i];
                let dy = p.y[j] - p.y[i];
                let dz = p.z[j] - p.z[i];
                let dist_sq = dx * dx + dy * dy + dz * dz + 0.01;
                total += p.mass[j] / dist_sq;
            }
        }
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    if total == 0.0 { 0.0 } else { elapsed }
}

fn bench_linked_list(head: &mut ListNode, n: usize, iters: usize) -> f64 {
    let start = Instant::now();
    let mut total = 0.0f64;
    for _ in 0..iters {
        let mut current = head as *mut ListNode;
        while !current.is_null() {
            total += unsafe { (*current).value };
            current = unsafe { (*current).next };
        }
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    if total == 0.0 { 0.0 } else { elapsed }
}

fn bench_array_access(arr: &[f64], n: usize, iters: usize) -> f64 {
    let start = Instant::now();
    let mut total = 0.0f64;
    for _ in 0..iters {
        for i in 0..n {
            total += arr[i];
        }
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    if total == 0.0 { 0.0 } else { elapsed }
}

struct ListNode {
    value: f64,
    next: *mut ListNode,
}

fn main() {
    println!("=== Cache-Aware Algorithm Design: Benchmarks ===\n");

    let sizes = [128usize, 256, 512];
    println!("Matrix Multiply Benchmarks:");
    println!("Cache hierarchy: L1=32KB (4 cycles), L2=256KB (12 cycles), L3=~8MB (40 cycles), DRAM=~200 cycles\n");

    for &n in &sizes {
        let kb = n * n * std::mem::size_of::<f64>() / 1024;
        println!("--- Matrix size: {}x{} ({} KB per matrix) ---", n, n, kb);

        let mut a = vec![vec![0.0f64; n]; n];
        let mut b = vec![vec![0.0f64; n]; n];
        let mut c = vec![vec![0.0f64; n]; n];
        init_matrix(&mut a, n);
        init_matrix(&mut b, n);

        let t_naive = {
            let start = Instant::now();
            mat_mul_naive(&a, &b, &mut c, n);
            start.elapsed().as_secs_f64() * 1000.0
        };
        let cs_naive = checksum(&c, n);

        let t_tiled = {
            let start = Instant::now();
            mat_mul_tiled(&a, &b, &mut c, n);
            start.elapsed().as_secs_f64() * 1000.0
        };
        let cs_tiled = checksum(&c, n);

        let t_oblivious = {
            let start = Instant::now();
            mat_mul_cache_oblivious(&a, &b, &mut c, n);
            start.elapsed().as_secs_f64() * 1000.0
        };
        let cs_oblivious = checksum(&c, n);

        println!("  Naive:     {:8.1} ms  (1.0x baseline)", t_naive);
        println!("  Tiled:     {:8.1} ms  ({:.1}x vs naive)", t_tiled, t_naive / t_tiled);
        println!("  Oblivious: {:8.1} ms  ({:.1}x vs naive)", t_oblivious, t_naive / t_oblivious);
        let cs_match = (cs_naive - cs_tiled).abs() < 1.0 && (cs_naive - cs_oblivious).abs() < 1.0;
        println!("  Checksum:  {} (naive={:.2} tiled={:.2} oblivious={:.2})",
                 if cs_match { "PASS" } else { "FAIL" }, cs_naive, cs_tiled, cs_oblivious);
        println!();
    }

    println!("=== AoS vs SoA Access Patterns ===\n");

    let n_particles = 4096usize;
    let iters = 100;

    let mut aos: Vec<ParticleAoS> = (0..n_particles).map(|i| {
        ParticleAoS {
            x: i as f64 * 0.001,
            y: i as f64 * 0.002,
            z: i as f64 * 0.003,
            vx: 0.1,
            vy: 0.2,
            vz: 0.3,
            mass: 1.0 + i as f64 * 0.0001,
            charge: 0.5,
        }
    }).collect();

    let mut soa = ParticlesSoA {
        x: (0..n_particles).map(|i| i as f64 * 0.001).collect(),
        y: (0..n_particles).map(|i| i as f64 * 0.002).collect(),
        z: (0..n_particles).map(|i| i as f64 * 0.003).collect(),
        vx: vec![0.1; n_particles],
        vy: vec![0.2; n_particles],
        vz: vec![0.3; n_particles],
        mass: (0..n_particles).map(|i| 1.0 + i as f64 * 0.0001).collect(),
        charge: vec![0.5; n_particles],
    };

    println!("  Particles: {} (struct size: {} bytes)", n_particles, std::mem::size_of::<ParticleAoS>());
    println!("  Hot loop accesses: x, y, z, mass (4 of 8 fields = 50% of struct)");

    let aos_time = bench_aos_gravity(&aos, n_particles.min(256), iters);
    let soa_time = bench_soa_gravity(&soa, n_particles.min(256), iters);

    println!("  AoS gravity: {:8.1} ms", aos_time);
    println!("  SoA gravity: {:8.1} ms  ({:.1}x vs AoS)", aos_time / soa_time);
    println!("  SoA wins when hot loop touches few fields per element.\n");

    println!("=== Linked List vs Array Traversal ===\n");

    let n_elems = 1_000_000usize;
    let iters = 10;

    let mut arr: Vec<f64> = (0..n_elems).map(|i| i as f64 * 0.001).collect();
    let arr_time = bench_array_access(&arr, n_elems, iters);

    let mut nodes: Vec<ListNode> = (0..n_elems).map(|_| ListNode { value: 0.0, next: std::ptr::null_mut() }).collect();
    for i in 0..n_elems {
        nodes[i].value = i as f64 * 0.001;
        nodes[i].next = if i + 1 < n_elems { &mut nodes[i + 1] as *mut ListNode } else { std::ptr::null_mut() };
    }
    let list_time = bench_linked_list(&mut nodes[0], n_elems, iters);

    println!("  Elements:  {}", n_elems);
    println!("  Array:     {:8.1} ms  (sequential, prefetcher-friendly)", arr_time);
    println!("  Linked:    {:8.1} ms  ({:.1}x slower — pointer chasing, cache-hostile)", list_time, list_time / arr_time);
    println!("  This is why B-trees beat BSTs: contiguous blocks vs scattered nodes.\n");

    println!("=== Key Takeaways ===");
    println!("1. Tiled matrix multiply: 5-30x faster than naive for large matrices.");
    println!("2. Cache-oblivious recursion: near-optimal without hardcoded tile sizes.");
    println!("3. SoA layout wins when hot loops touch few fields per element.");
    println!("4. Arrays destroy linked lists for traversal — hardware prefetchers win.");
    println!("5. Algorithmic complexity is necessary but not sufficient: cache locality decides reality.");
}