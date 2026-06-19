// ════════════════════════════════════════════════════════════════════
// WGSL Compute Shaders — GPU Programming with WebGPU
// Phase 13, Lesson 20
// ════════════════════════════════════════════════════════════════════
//
// This file contains two compute shader entry points:
//   1. vec_add    — element-wise vector addition      (Step 2)
//   2. matmul_tiled — tiled matrix multiplication      (Step 3)
//      with workgroup shared memory
//
// Compile: WebGPU compiles WGSL at runtime via
//   device.createShaderModule({ code })
//
// ════════════════════════════════════════════════════════════════════

// ──────────────────────────────────────────────────────────────────
// Step 2: Vector Addition
// ──────────────────────────────────────────────────────────────────
// Each invocation adds one element: c[i] = a[i] + b[i].
// Workgroup size: 256 (one-dimensional grid).
//
// Binding layout:
//   @group(0) @binding(0) — input  array a (read)
//   @group(0) @binding(1) — input  array b (read)
//   @group(0) @binding(2) — output array c (read_write)
// ──────────────────────────────────────────────────────────────────

@group(0) @binding(0) var<storage, read>     a: array<f32>;
@group(0) @binding(1) var<storage, read>     b: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;

@compute @workgroup_size(256)
fn vec_add(@builtin(global_invocation_id) id: vec3u) {
    let idx = id.x;
    let n = arrayLength(&a);
    if (idx >= n) {
        return;
    }
    c[idx] = a[idx] + b[idx];
}

// ──────────────────────────────────────────────────────────────────
// Step 3: Tiled Matrix Multiplication
// ──────────────────────────────────────────────────────────────────
// C = A × B  (square N×N matrices stored in column-major order).
//
// Uses workgroup shared memory (tileA, tileB) to reduce global
// memory traffic from O(N³) to O(N³ / TILE_SIZE).
//
// Each workgroup (16×16 = 256 invocations) loads one 16×16 tile
// from A and one from B into shared memory, computes partial
// products, then iterates across the K dimension.
//
// Binding layout:
//   @group(0) @binding(0) — input matrix A (read)
//   @group(0) @binding(1) — input matrix B (read)
//   @group(0) @binding(2) — output matrix C (read_write)
// ──────────────────────────────────────────────────────────────────

const TILE_SIZE: u32 = 16u;

var<workgroup> tileA: array<array<f32, TILE_SIZE>, TILE_SIZE>;
var<workgroup> tileB: array<array<f32, TILE_SIZE>, TILE_SIZE>;

@group(0) @binding(0) var<storage, read>     A: array<f32>;
@group(0) @binding(1) var<storage, read>     B: array<f32>;
@group(0) @binding(2) var<storage, read_write> C: array<f32>;

@compute @workgroup_size(TILE_SIZE, TILE_SIZE, 1)
fn matmul_tiled(
    @builtin(global_invocation_id) gid: vec3u,
    @builtin(local_invocation_id) lid: vec3u,
) {
    // Derive N from the size of the output buffer (assumes square).
    let N: u32 = u32(sqrt(f32(arrayLength(&C))));

    let col: u32 = gid.x;
    let row: u32 = gid.y;

    if (col >= N || row >= N) {
        return;
    }

    let numTiles: u32 = (N + TILE_SIZE - 1u) / TILE_SIZE;
    var sum: f32 = 0.0;

    for (var t: u32 = 0u; t < numTiles; t = t + 1u) {
        // Each invocation loads one element of tileA and one of tileB.
        let tcol: u32 = t * TILE_SIZE + lid.x;
        let trow: u32 = t * TILE_SIZE + lid.y;

        if (tcol < N && trow < N) {
            // A[row][t*TILE_SIZE + lid.x]
            tileA[lid.y][lid.x] = A[trow * N + (t * TILE_SIZE + lid.x)];
            // B[(t*TILE_SIZE + lid.y)][col]
            tileB[lid.y][lid.x] = B[(t * TILE_SIZE + lid.y) * N + col];
        } else {
            tileA[lid.y][lid.x] = 0.0;
            tileB[lid.y][lid.x] = 0.0;
        }

        // Ensure all invocations have finished loading before computing.
        workgroupBarrier();

        // Compute partial product for this tile.
        for (var k: u32 = 0u; k < TILE_SIZE; k = k + 1u) {
            sum = sum + tileA[lid.y][k] * tileB[k][lid.x];
        }

        // Ensure no invocation reads a tile that another is about to overwrite.
        workgroupBarrier();
    }

    C[row * N + col] = sum;
}

// ──────────────────────────────────────────────────────────────────
// Utility: arrayLength usage
//   - arrayLength(&buffer) returns the number of elements in a
//     storage buffer array. This is the WGSL idiomatic way to
//     handle variable-length buffers without separate uniforms.
// ──────────────────────────────────────────────────────────────────

// ──────────────────────────────────────────────────────────────────
// Note: Barrier semantics
//   workgroupBarrier() is a "full" barrier (memory + execution).
//   It ensures that all preceding memory accesses in the workgroup
//   are visible to all invocations before any subsequent access.
//
//   Equivalent to __syncthreads() in CUDA.
//
//   Unlike CUDA, WGSL does NOT have warp-level primitives
//   (__shfl_sync, __ballot_sync, etc.) in the current spec.
// ──────────────────────────────────────────────────────────────────
