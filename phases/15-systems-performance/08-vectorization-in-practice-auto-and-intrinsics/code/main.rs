use std::arch::x86_64::*;
use std::time::Instant;

const N: usize = 1024 * 1024;
const ITERS: usize = 200;

fn alloc_aligned(n: usize, alignment: usize) -> Vec<f32> {
    let layout = std::alloc::Layout::from_size_align(n * std::mem::size_of::<f32>(), alignment).unwrap();
    unsafe {
        let ptr = std::alloc::alloc(layout) as *mut f32;
        for i in 0..n {
            ptr.add(i).write(0.0f32);
        }
        Vec::from_raw_parts(ptr, n, n)
    }
}

fn fill_random(data: &mut [f32]) {
    let mut rng_state: u32 = 12345;
    for i in 0..data.len() {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        data[i] = (rng_state as f32) / (u32::MAX as f32) - 0.5;
    }
}

// ─── Scalar implementations ───

fn dot_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

fn sum_scalar(data: &[f32]) -> f32 {
    let mut s = 0.0f32;
    for i in 0..data.len() {
        s += data[i];
    }
    s
}

fn filter_scalar(src: &[f32], dst: &mut [f32], threshold: f32) -> usize {
    let mut count = 0;
    for i in 0..src.len() {
        if src[i] > threshold {
            dst[count] = src[i];
            count += 1;
        }
    }
    count
}

// ─── SSE intrinsics (128-bit, 4 × float32) ───

#[target_feature(enable = "sse")]
unsafe fn dot_sse(a: &[f32], b: &[f32]) -> f32 {
    let mut sum_vec = _mm_setzero_ps();
    let n = a.len();
    let mut i = 0;
    while i + 4 <= n {
        let va = _mm_loadu_ps(a.as_ptr().add(i));
        let vb = _mm_loadu_ps(b.as_ptr().add(i));
        sum_vec = _mm_add_ps(sum_vec, _mm_mul_ps(va, vb));
        i += 4;
    }
    let mut tmp = [0.0f32; 4];
    _mm_storeu_ps(tmp.as_mut_ptr(), sum_vec);
    let mut sum = tmp[0] + tmp[1] + tmp[2] + tmp[3];
    while i < n {
        sum += a[i] * b[i];
        i += 1;
    }
    sum
}

#[target_feature(enable = "sse")]
unsafe fn sum_sse(data: &[f32]) -> f32 {
    let mut sum_vec = _mm_setzero_ps();
    let n = data.len();
    let mut i = 0;
    while i + 4 <= n {
        let v = _mm_loadu_ps(data.as_ptr().add(i));
        sum_vec = _mm_add_ps(sum_vec, v);
        i += 4;
    }
    let mut tmp = [0.0f32; 4];
    _mm_storeu_ps(tmp.as_mut_ptr(), sum_vec);
    let mut sum = tmp[0] + tmp[1] + tmp[2] + tmp[3];
    while i < n {
        sum += data[i];
        i += 1;
    }
    sum
}

// ─── AVX intrinsics (256-bit, 8 × float32) ───

#[target_feature(enable = "avx")]
unsafe fn dot_avx(a: &[f32], b: &[f32]) -> f32 {
    let mut sum_vec = _mm256_setzero_ps();
    let n = a.len();
    let mut i = 0;
    while i + 8 <= n {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));
        sum_vec = _mm256_add_ps(sum_vec, _mm256_mul_ps(va, vb));
        i += 8;
    }
    let mut tmp = [0.0f32; 8];
    _mm256_storeu_ps(tmp.as_mut_ptr(), sum_vec);
    let mut sum = 0.0f32;
    for j in 0..8 {
        sum += tmp[j];
    }
    while i < n {
        sum += a[i] * b[i];
        i += 1;
    }
    sum
}

#[target_feature(enable = "avx")]
unsafe fn sum_avx(data: &[f32]) -> f32 {
    let mut sum_vec = _mm256_setzero_ps();
    let n = data.len();
    let mut i = 0;
    while i + 8 <= n {
        let v = _mm256_loadu_ps(data.as_ptr().add(i));
        sum_vec = _mm256_add_ps(sum_vec, v);
        i += 8;
    }
    let mut tmp = [0.0f32; 8];
    _mm256_storeu_ps(tmp.as_mut_ptr(), sum_vec);
    let mut sum = 0.0f32;
    for j in 0..8 {
        sum += tmp[j];
    }
    while i < n {
        sum += data[i];
        i += 1;
    }
    sum
}

// ─── AVX + FMA intrinsics (fused multiply-add) ───

#[target_feature(enable = "avx2,fma")]
unsafe fn dot_avx_fma(a: &[f32], b: &[f32]) -> f32 {
    let mut sum_vec = _mm256_setzero_ps();
    let n = a.len();
    let mut i = 0;
    while i + 8 <= n {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));
        sum_vec = _mm256_fmadd_ps(va, vb, sum_vec);
        i += 8;
    }
    let mut tmp = [0.0f32; 8];
    _mm256_storeu_ps(tmp.as_mut_ptr(), sum_vec);
    let mut sum = 0.0f32;
    for j in 0..8 {
        sum += tmp[j];
    }
    while i < n {
        sum += a[i] * b[i];
        i += 1;
    }
    sum
}

// ─── AVX2 filter: copy src[i] > threshold to dst, return count ───

#[target_feature(enable = "avx2")]
unsafe fn filter_avx2(src: &[f32], dst: &mut [f32], threshold: f32) -> usize {
    let thresh = _mm256_set1_ps(threshold);
    let n = src.len();
    let mut count: usize = 0;
    let mut i = 0;
    while i + 8 <= n {
        let v = _mm256_loadu_ps(src.as_ptr().add(i));
        let cmp = _mm256_cmp_ps(v, thresh, _CMP_GT_OS);
        let mask = _mm256_movemask_ps(cmp) as u32;
        let mut m = mask;
        while m != 0 {
            let bit = m.trailing_zeros() as usize;
            dst[count] = src[i + bit];
            count += 1;
            m &= m - 1;
        }
        i += 8;
    }
    while i < n {
        if src[i] > threshold {
            dst[count] = src[i];
            count += 1;
        }
        i += 1;
    }
    count
}

// ─── Aligned AVX load benchmark ───

#[target_feature(enable = "avx")]
unsafe fn sum_aligned_avx(data: &[f32]) -> f32 {
    let mut sum_vec = _mm256_setzero_ps();
    let n = data.len();
    let mut i = 0;
    while i + 8 <= n {
        let v = _mm256_load_ps(data.as_ptr().add(i));
        sum_vec = _mm256_add_ps(sum_vec, v);
        i += 8;
    }
    let mut tmp = [0.0f32; 8];
    _mm256_storeu_ps(tmp.as_mut_ptr(), sum_vec);
    let mut sum = 0.0f32;
    for j in 0..8 {
        sum += tmp[j];
    }
    while i < n {
        sum += data[i];
        i += 1;
    }
    sum
}

// ─── Timing helper ───

fn bench<F: Fn() -> f32>(label: &str, func: F) -> f64 {
    let start = Instant::now();
    let mut sink = 0.0f32;
    for _ in 0..ITERS {
        sink = func();
    }
    std::hint::black_box(sink);
    let elapsed = start.elapsed().as_micros() as f64 / ITERS as f64;
    println!("{:<20}: {:.1} us/iter", label, elapsed);
    elapsed
}

fn bench_us<F: Fn() -> usize>(label: &str, func: F) -> f64 {
    let start = Instant::now();
    let mut sink = 0usize;
    for _ in 0..ITERS {
        sink = func();
    }
    std::hint::black_box(sink);
    let elapsed = start.elapsed().as_micros() as f64 / ITERS as f64;
    println!("{:<20}: {:.1} us/iter", label, elapsed);
    elapsed
}

fn main() {
    println!("=== Vectorization Benchmark (Rust) ===");
    println!("Array size: {} floats ({} KB)\n", N, N * std::mem::size_of::<f32>() / 1024);

    let mut a = vec![0.0f32; N];
    let mut b = vec![0.0f32; N];
    let mut dst = vec![0.0f32; N];
    fill_random(&mut a);
    fill_random(&mut b);

    // ─── Dot Product Benchmarks ───
    println!("--- Dot Product ---");
    let ref_val = dot_scalar(&a, &b);

    bench("scalar", || dot_scalar(&a, &b));

    unsafe {
        if is_x86_feature_detected!("sse") {
            bench("SSE", || dot_sse(&a, &b));
        } else {
            println!("SSE                  : (not available)");
        }

        if is_x86_feature_detected!("avx") {
            bench("AVX", || dot_avx(&a, &b));
        } else {
            println!("AVX                  : (not available)");
        }

        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            bench("AVX+FMA", || dot_avx_fma(&a, &b));
        } else {
            println!("AVX+FMA              : (not available)");
        }
    }

    // ─── Sum Benchmarks ───
    println!("\n--- Sum ---");

    bench("scalar", || sum_scalar(&a));

    unsafe {
        if is_x86_feature_detected!("sse") {
            bench("SSE", || sum_sse(&a));
        }
        if is_x86_feature_detected!("avx") {
            bench("AVX (unaligned)", || sum_avx(&a));
            bench("AVX (aligned)", || sum_aligned_avx(&a));
        }
    }

    // ─── Filter Benchmark ───
    println!("\n--- Filter (> 0.0) ---");
    let threshold = 0.0f32;

    bench_us("scalar", || filter_scalar(&a, &mut dst, threshold));

    unsafe {
        if is_x86_feature_detected!("avx2") {
            bench_us("AVX2", || filter_avx2(&a, &mut dst, threshold));
        } else {
            println!("AVX2                 : (not available)");
        }
    }

    // ─── Verification ───
    println!("\n--- Verification ---");
    let mut all_ok = true;

    let d_auto = dot_scalar(&a, &b);
    if (d_auto - ref_val).abs() > 1e-3 { println!("FAIL: auto != scalar"); all_ok = false; }

    unsafe {
        if is_x86_feature_detected!("sse") {
            let d_sse = dot_sse(&a, &b);
            if (d_sse - ref_val).abs() > 1e-3 { println!("FAIL: SSE != scalar"); all_ok = false; }
        }
        if is_x86_feature_detected!("avx") {
            let d_avx = dot_avx(&a, &b);
            if (d_avx - ref_val).abs() > 1e-3 { println!("FAIL: AVX != scalar"); all_ok = false; }
        }
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            let d_fma = dot_avx_fma(&a, &b);
            if (d_fma - ref_val).abs() > 1e-3 { println!("FAIL: FMA != scalar"); all_ok = false; }
        }
    }

    if all_ok { println!("All dot product results match (within 1e-3)."); }

    let s_scalar = sum_scalar(&a);
    unsafe {
        if is_x86_feature_detected!("avx") {
            let s_avx = sum_avx(&a);
            if (s_avx - s_scalar).abs() > 1e-2 { println!("FAIL: sum AVX != scalar"); }
            else { println!("All sum results match (within 1e-2)."); }
        }
    }

    let cnt_scalar = filter_scalar(&a, &mut dst, threshold);
    unsafe {
        if is_x86_feature_detected!("avx2") {
            let cnt_avx2 = filter_avx2(&a, &mut dst, threshold);
            if cnt_scalar == cnt_avx2 {
                println!("Filter counts match: {} elements.", cnt_scalar);
            } else {
                println!("FAIL: filter counts differ: scalar={} avx2={}", cnt_scalar, cnt_avx2);
            }
        }
    }
}