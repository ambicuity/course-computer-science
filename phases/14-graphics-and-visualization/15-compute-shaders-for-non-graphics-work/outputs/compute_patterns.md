# GPU Compute Patterns — Quick Reference Card

## Pattern 1: Parallel Reduction

**Purpose:** Combine all elements into one value (sum, max, min, any, all).  
**Depth:** O(log n)  
**Work:** O(n)

### CUDA

```cuda
__global__ void reduce_sum(float* data, float* result, int n) {
    __shared__ float s[256];
    int tid = threadIdx.x;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    s[tid] = (i < n) ? data[i] : 0.0f;
    __syncthreads();
    for (int stride = blockDim.x / 2; stride > 0; stride >>= 1) {
        if (tid < stride) s[tid] += s[tid + stride];
        __syncthreads();
    }
    if (tid == 0) atomicAdd(result, s[0]);
}
```

### WGSL

```wgsl
var<workgroup> shared_buf: array<f32, 256>;

@compute @workgroup_size(256)
fn reduce_sum(@builtin(local_invocation_id) local_id: vec3u,
              @builtin(global_id) global_id: vec3u) {
    let tid = local_id.x;
    let i = global_id.x;
    shared_buf[tid] = (i < n) ? input.data[i] : 0.0;
    workgroupBarrier();
    var stride: u32 = 128u;
    while (stride > 0u) {
        if (tid < stride) {
            shared_buf[tid] = shared_buf[tid] + shared_buf[tid + stride];
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }
    if (tid == 0u) { result.value = shared_buf[0]; }
}
```

### Key Points

- Halves active threads each step: stride = blockDim/2 → 1
- **Always** call `__syncthreads()` / `workgroupBarrier()` after shared memory writes
- For max/min: replace `+=` with `fmaxf`/`fminf`
- Multi-block: each block writes partial result, second pass reduces those

---

## Pattern 2: Blelloch Prefix Sum (Exclusive Scan)

**Purpose:** Compute all running totals [x₀, x₀+x₁, x₀+x₁+x₂, ...]  
**Depth:** O(log n)  
**Work:** O(n)

### CUDA

```cuda
__global__ void blelloch_scan(float* input, float* output, int n) {
    __shared__ float temp[512];  // 2 * BLOCK_SIZE
    int tid = threadIdx.x;
    // Load: temp[ai] = input[blockOffset + ai], temp[bi] = input[...]
    // UP-SWEEP: reduce pairs at increasing stride
    int offset = 1;
    for (int d = BLOCK_SIZE >> 1; d > 0; d >>= 1) {
        if (tid < d) {
            int a = offset * (2 * tid + 1) - 1;
            int b = offset * (2 * tid + 2) - 1;
            temp[b] += temp[a];
        }
        offset *= 2;
        __syncthreads();
    }
    // Set last element to 0 (exclusive scan)
    if (tid == 0) temp[BLOCK_SIZE - 1] = 0.0f;
    __syncthreads();
    // DOWN-SWEEP: distribute partial sums at decreasing stride
    for (int d = 1; d < BLOCK_SIZE; d *= 2) {
        offset >>= 1;
        if (tid < d) {
            int a = offset * (2 * tid + 1) - 1;
            int b = offset * (2 * tid + 2) - 1;
            float t = temp[a];
            temp[a] = temp[b];
            temp[b] += t;
        }
        __syncthreads();
    }
    // Store: output[blockOffset + ai] = temp[ai], etc.
}
```

### WGSL

```wgsl
var<workgroup> scan_temp: array<f32, 512>;

@compute @workgroup_size(256)
fn blelloch_scan(@builtin(local_invocation_id) local_id: vec3u) {
    let tid = local_id.x;
    // Load data into scan_temp[ai] and scan_temp[bi]
    // UP-SWEEP: same indexing logic as CUDA
    var offset: u32 = 1u;
    var d: u32 = 128u;
    while (d > 0u) {
        if (tid < d) {
            let a = offset * (2u * tid + 1u) - 1u;
            let b = offset * (2u * tid + 2u) - 1u;
            scan_temp[b] = scan_temp[b] + scan_temp[a];
        }
        offset = offset * 2u;
        workgroupBarrier();
        d = d >> 1u;
    }
    if (tid == 0u) { scan_temp[511u] = 0.0; }
    workgroupBarrier();
    // DOWN-SWEEP: distribute
    d = 1u;
    while (d < 256u) {
        offset = offset >> 1u;
        if (tid < d) {
            let a = offset * (2u * tid + 1u) - 1u;
            let b = offset * (2u * tid + 2u) - 1u;
            let t = scan_temp[a];
            scan_temp[a] = scan_temp[b];
            scan_temp[b] = scan_temp[b] + t;
        }
        workgroupBarrier();
        d = d * 2u;
    }
    // Write output from scan_temp
}
```

### Key Points

- Requires 2× workspace: `temp[2 * BLOCK_SIZE]`
- Up-sweep = reduction phase (builds partial sums)
- Down-sweep = distribution phase (spreads partial sums into positions)
- Result is **exclusive** prefix sum: output[i] = sum of input[0..i)
- For inclusive scan, add input[i] to each output[i] at the end

---

## Pattern 3: Bitonic Merge Sort

**Purpose:** Sort n = 2^k elements on GPU.  
**Depth:** O(log² n)  
**Work:** O(n log² n)

### Pseudocode (CUDA-style)

```cuda
__global__ void bitonic_sort(float* data, int n) {
    __shared__ float s[256];
    int tid = threadIdx.x;
    if (tid < n) s[tid] = data[tid];
    __syncthreads();

    for (int k = 2; k <= n; k <<= 1) {           // merge size
        for (int j = k >> 1; j > 0; j >>= 1) {   // compare distance
            int ij = tid ^ j;                     // partner index
            if (ij > tid) {
                bool ascending = (tid & k) == 0;
                if ((s[tid] > s[ij]) == ascending) {
                    float tmp = s[tid];
                    s[tid] = s[ij];
                    s[ij] = tmp;
                }
            }
            __syncthreads();
        }
    }
    if (tid < n) data[tid] = s[tid];
}
```

### Key Points

- Input must be power-of-2 (pad with infinity if not)
- Each merge step doubles the sorted subsequence length
- Compare-and-swap direction alternates (bitonic property)
- For WGSL: same logic, use `var<workgroup>` for shared array
- For large arrays: multi-workgroup sort with global-memory stages

---

## CUDA ↔ WGSL Quick Translation

| Concept            | CUDA                              | WGSL                                   |
|--------------------|-----------------------------------|----------------------------------------|
| Kernel             | `__global__ void k(...)`          | `@compute fn k(...)`                   |
| Workgroup size     | `<<<grid, block>>>`               | `@workgroup_size(256)`                 |
| Block/workgroup ID | `blockIdx.x`                      | `@builtin(workgroup_id).x`            |
| Local thread ID    | `threadIdx.x`                     | `@builtin(local_invocation_id).x`      |
| Global thread ID   | `blockIdx.x*blockDim.x+threadIdx.x` | `@builtin(global_id).x`              |
| Shared memory      | `__shared__ float s[N]`           | `var<workgroup> s: array<f32, N>`     |
| Barrier            | `__syncthreads()`                 | `workgroupBarrier()`                   |
| Atomic add         | `atomicAdd(&ptr, val)`            | `atomicAdd(&ptr, val)`                 |
| Storage buffer     | Pointer arg                       | `@binding(N) var<storage, read_write>` |
| Read-only buffer   | Pointer arg                       | `@binding(N) var<storage, read>`       |
| Loop: halve stride | `stride >>= 1`                    | `stride = stride >> 1u`               |

---

## When to Use Each Pattern

| Pattern         | Use When                                    | Example Applications                    |
|-----------------|---------------------------------------------|----------------------------------------|
| Reduction       | You need one value from many                 | Sum, max, min, dot product             |
| Prefix Sum      | You need all running totals or compaction   | Stream compaction, sort, CDF           |
| Bitonic Sort    | You need GPU-side sorting of moderate data  | Particle sort, BVH construction, depth |