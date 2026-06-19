// SIMD Programming in Practice
// Phase 13 — Concurrent & Parallel Computing, Lesson 18
//
// Requires nightly Rust with `#![feature(portable_simd)]`
//
// Build:   rustup default nightly
//          cargo build --release
// Run:     cargo run --release

#![feature(portable_simd)]

use std::simd::{f32x4, f32x8, Simd};
use std::time::Instant;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

/// Time a closure `trials` times and return the best (fastest) duration in
/// microseconds.
fn time_it<F: Fn()>(f: F, trials: u32) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..trials {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed().as_secs_f64() * 1_000_000.0;
        best = best.min(elapsed);
    }
    best
}

fn make_vec(n: usize) -> Vec<f32> {
    // Use a simple LCG for reproducibility; avoid `rand` crate dependency.
    let mut v = Vec::with_capacity(n);
    let mut state: u32 = 42;
    for _ in 0..n {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_011_904_223);
        v.push((state >> 8) as f32 / (1u32 << 24) as f32);
    }
    v
}

// -----------------------------------------------------------------------
// Section 1 — Scalar (baseline)
// -----------------------------------------------------------------------

fn add_scalar(a: &[f32], b: &[f32], c: &mut [f32]) {
    for i in 0..a.len() {
        c[i] = a[i] + b[i];
    }
}

fn dot_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

fn matmul_4x4_scalar(a: &[f32; 16], b: &[f32; 16], c: &mut [f32; 16]) {
    for i in 0..4 {
        for j in 0..4 {
            let mut sum = 0.0f32;
            for k in 0..4 {
                sum += a[i * 4 + k] * b[k * 4 + j];
            }
            c[i * 4 + j] = sum;
        }
    }
}

// -----------------------------------------------------------------------
// Section 2 — Rust Portable SIMD  (std::simd, 256-bit via f32x8)
// -----------------------------------------------------------------------

fn add_simd(a: &[f32], b: &[f32], c: &mut [f32]) {
    let mut i = 0;
    while i + 8 <= a.len() {
        let va = f32x8::from_slice(&a[i..]);
        let vb = f32x8::from_slice(&b[i..]);
        (va + vb).copy_to_slice(&mut c[i..]);
        i += 8;
    }
    // Tail
    for j in i..a.len() {
        c[j] = a[j] + b[j];
    }
}

fn dot_simd(a: &[f32], b: &[f32]) -> f32 {
    let mut vsum = f32x8::splat(0.0);
    let mut i = 0;
    while i + 8 <= a.len() {
        let va = f32x8::from_slice(&a[i..]);
        let vb = f32x8::from_slice(&b[i..]);
        vsum = vsum + (va * vb);
        i += 8;
    }
    let mut total = vsum.reduce_sum();
    for j in i..a.len() {
        total += a[j] * b[j];
    }
    total
}

fn matmul_4x4_simd(a: &[f32; 16], b: &[f32; 16], c: &mut [f32; 16]) {
    // Load B as column vectors (transposed view)
    let b_col0 = f32x4::from_array([b[0], b[4], b[8], b[12]]);
    let b_col1 = f32x4::from_array([b[1], b[5], b[9], b[13]]);
    let b_col2 = f32x4::from_array([b[2], b[6], b[10], b[14]]);
    let b_col3 = f32x4::from_array([b[3], b[7], b[11], b[15]]);

    for i in 0..4 {
        let a0 = f32x4::splat(a[i * 4 + 0]);
        let a1 = f32x4::splat(a[i * 4 + 1]);
        let a2 = f32x4::splat(a[i * 4 + 2]);
        let a3 = f32x4::splat(a[i * 4 + 3]);

        let crow = a0 * b_col0 + a1 * b_col1 + a2 * b_col2 + a3 * b_col3;
        // Write row i
        c[i * 4 + 0] = crow[0];
        c[i * 4 + 1] = crow[1];
        c[i * 4 + 2] = crow[2];
        c[i * 4 + 3] = crow[3];
    }
}

// -- 2b. Sum (pure reduction, no multiply) -----------------------------

fn sum_scalar(a: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..a.len() {
        sum += a[i];
    }
    sum
}

fn sum_simd(a: &[f32]) -> f32 {
    let mut vsum = f32x8::splat(0.0);
    let mut i = 0;
    while i + 8 <= a.len() {
        let va = f32x8::from_slice(&a[i..]);
        vsum = vsum + va;
        i += 8;
    }
    let mut total = vsum.reduce_sum();
    for j in i..a.len() {
        total += a[j];
    }
    total
}

// -- 2c. Strided add (non-unit stride) ---------------------------------
//
// Non-unit stride prevents efficient SIMD for stores.  We show both
// the scalar version and a gather-based SIMD version; the latter is
// often slower due to memory-latency overhead.

fn add_strided_scalar(a: &[f32], b: &[f32], c: &mut [f32], stride: usize) {
    for i in 0..a.len() {
        c[i * stride] = a[i] + b[i];
    }
}

fn add_strided_simd(a: &[f32], b: &[f32], c: &mut [f32], stride: usize) {
    let stride_i32 = stride as i32;
    let mut i = 0;
    while i + 8 <= a.len() {
        // Build index vector: [i, i+stride, i+2*stride, ...]
        // Indices*4 (byte offset) needed for gather, but we use
        // element indexing so multiply by sizeof(f32) at use site.
        let base = f32x8::splat(i as f32);
        let step = f32x8::splat(stride as f32);
        // Simulate indexed gather: f32x8 does not natively support
        // gather, so we fall back to a loop for this case:
        // Rust portable SIMD does not expose gather intrinsics.
        // This demonstrates the *limitation* of portable abstraction.
        for k in 0..8 {
            c[(i + k) * stride] = a[i + k] + b[i + k];
        }
        i += 8;
    }
    for j in i..a.len() {
        c[j * stride] = a[j] + b[j];
    }
}

// -- 2d. f32x4 vs f32x8 comparison --------------------------------------
//
// f32x4 (128-bit, 4 lanes) matches SSE width; f32x8 (256-bit, 8 lanes)
// matches AVX2 width.  Running the same problem with both widths
// demonstrates the advantage of wider vectors (when memory bandwidth
// is not the bottleneck).

fn add_simd_128(a: &[f32], b: &[f32], c: &mut [f32]) {
    let mut i = 0;
    while i + 4 <= a.len() {
        let va = f32x4::from_slice(&a[i..]);
        let vb = f32x4::from_slice(&b[i..]);
        (va + vb).copy_to_slice(&mut c[i..]);
        i += 4;
    }
    for j in i..a.len() {
        c[j] = a[j] + b[j];
    }
}

// -----------------------------------------------------------------------
// Section 3 — Benchmark runner
// -----------------------------------------------------------------------

const N: usize = 8 << 20; // 8 million elements

fn main() {
    println!("\n=== Rust SIMD Benchmark  (N = {N} floats) ===\n");

    let a = make_vec(N);
    let b = make_vec(N);
    let mut c = vec![0.0f32; N];
    let mut d = vec![0.0f32; N];

    // ---- Element-wise Add --------------------------------------------
    {
        let t_scalar = time_it(|| add_scalar(&a, &b, &mut c), 5);
        let t_simd = time_it(|| add_simd(&a, &b, &mut d), 5);

        let ok = c.iter().zip(d.iter()).all(|(x, y)| (x - y).abs() < 1e-4);

        println!("--- Element-wise Add ---");
        println!("  scalar  : {t_scalar:.3} us");
        println!("  simd    : {t_simd:.3} us  ({:.1}x)", t_scalar / t_simd);
        println!("  correct : {}\n", if ok { "yes" } else { "FAIL" });
    }

    // ---- Dot Product -------------------------------------------------
    {
        let t_scalar = time_it(|| { dot_scalar(&a, &b); }, 5);
        let t_simd = time_it(|| { dot_simd(&a, &b); }, 5);

        let r_scalar = dot_scalar(&a, &b);
        let r_simd = dot_simd(&a, &b);
        let err = (r_scalar - r_simd).abs();

        println!("--- Dot Product ---");
        println!("  scalar  : {t_scalar:.3} us  (result {r_scalar})");
        println!("  simd    : {t_simd:.3} us  ({:.1}x)", t_scalar / t_simd);
        println!("  error   : {err:e}\n");
    }

    // ---- 4x4 Matrix Multiply -----------------------------------------
    {
        let ma = make_vec(16);
        let mb = make_vec(16);
        let mut c_ref = [0.0f32; 16];
        let mut c_simd = [0.0f32; 16];

        let a_arr: [f32; 16] = ma.try_into().unwrap();
        let b_arr: [f32; 16] = mb.try_into().unwrap();

        let t_scalar =
            time_it(|| matmul_4x4_scalar(&a_arr, &b_arr, &mut c_ref), 5);
        let t_simd =
            time_it(|| matmul_4x4_simd(&a_arr, &b_arr, &mut c_simd), 5);

        let ok = c_ref
            .iter()
            .zip(c_simd.iter())
            .all(|(x, y)| (x - y).abs() < 1e-4);

        println!("--- 4x4 Matrix Multiply ---");
        println!("  scalar  : {t_scalar:.3} us");
        println!("  simd    : {t_simd:.3} us  ({:.1}x)", t_scalar / t_simd);
        println!("  correct : {}\n", if ok { "yes" } else { "FAIL" });
    }

    // ---- Sum (pure reduction) -----------------------------------------
    {
        let t_scalar = time_it(|| { sum_scalar(&a); }, 5);
        let t_simd = time_it(|| { sum_simd(&a); }, 5);

        let r_scalar = sum_scalar(&a);
        let r_simd = sum_simd(&a);
        let err = (r_scalar - r_simd).abs();

        println!("--- Sum (pure reduction) ---");
        println!("  scalar  : {t_scalar:.3} us  (result {r_scalar})");
        println!("  simd    : {t_simd:.3} us  ({:.1}x)", t_scalar / t_simd);
        println!("  error   : {err:e}\n");
    }

    // ---- Strided add (stride=4) ---------------------------------------
    {
        let stride = 4;
        let stride_n = N / 16;
        let mut c_strided = vec![0.0f32; stride_n * stride];

        let t_scalar = time_it(
            || add_strided_scalar(&a[..stride_n], &b[..stride_n],
                                  &mut c_strided, stride),
            5,
        );
        let t_simd = time_it(
            || add_strided_simd(&a[..stride_n], &b[..stride_n],
                                &mut c_strided, stride),
            5,
        );

        let ok = (0..stride_n).all(|i| {
            (c_strided[i * stride] - (a[i] + b[i])).abs() < 1e-4
        });

        println!("--- Strided Add (stride=4) ---");
        println!("  scalar  : {t_scalar:.3} us");
        println!("  simd    : {t_simd:.3} us  ({:.1}x)", t_scalar / t_simd);
        println!("  correct : {}", if ok { "yes" } else { "FAIL" });
        println!("  (gather not exposed by portable SIMD;\n\
                   both versions use scalar tail)\n");
    }

    // ---- f32x4 vs f32x8 width comparison ------------------------------
    {
        // Use a smaller N so data fits in L2 cache, highlighting
        // compute throughput rather than memory bandwidth.
        let small_n = 1 << 18; // 256 KiB
        let a_small = make_vec(small_n);
        let b_small = make_vec(small_n);
        let mut c128 = vec![0.0f32; small_n];
        let mut c256 = vec![0.0f32; small_n];

        let t_128 = time_it(
            || add_simd_128(&a_small, &b_small, &mut c128), 5);
        let t_256 = time_it(
            || add_simd(&a_small, &b_small, &mut c256), 5);

        let ok = c128.iter().zip(c256.iter())
            .all(|(x, y)| (x - y).abs() < 1e-4);

        println!("--- Width Comparison  (N = {small_n}, L2-resident) ---");
        println!("  f32x4 (128-bit): {t_128:.3} us");
        println!("  f32x8 (256-bit): {t_256:.3} us  ({:.1}x)",
                 t_128 / t_256);
        println!("  correct: {}\n", if ok { "yes" } else { "FAIL" });
    }

    // ---- Summary -----------------------------------------------------
    println!("--- Summary ---");
    println!("Rust portable SIMD targets the widest available ISA.");
    println!("On x86 with AVX2:  f32x8 = 256 bits = 8 × f32 lanes.");
    println!("On AArch64:       f32x8 lowered to NEON (2×128-bit ops).");
    println!("f32x4 vs f32x8 comparison shows wider vectors give\n\
              ~1.5-2x speedup when compute-bound (L2-resident).\n\
              Memory-bound workloads see less gain.\n");
}
