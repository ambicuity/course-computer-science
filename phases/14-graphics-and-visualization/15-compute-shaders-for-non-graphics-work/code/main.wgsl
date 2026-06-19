struct ReductionInput {
    data: array<f32>,
}

struct ReductionResult {
    value: f32,
}

@group(0) @binding(0) var<storage, read> input: ReductionInput;
@group(0) @binding(1) var<storage, read_write> result: ReductionResult;

var<workgroup> shared_buf: array<f32, 256>;

@compute @workgroup_size(256)
fn reduce_sum(@builtin(local_invocation_id) local_id: vec3u, @builtin(global_id) global_id: vec3u) {
    let tid: u32 = local_id.x;
    let i: u32 = global_id.x;
    let n: u32 = arrayLength(&input.data);

    if (i < n) {
        shared_buf[tid] = input.data[i];
    } else {
        shared_buf[tid] = 0.0;
    }
    workgroupBarrier();

    var stride: u32 = 128u;
    while (stride > 0u) {
        if (tid < stride) {
            shared_buf[tid] = shared_buf[tid] + shared_buf[tid + stride];
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }

    if (tid == 0u) {
        result.value = shared_buf[0];
    }
}

var<workgroup> shared_max_buf: array<f32, 256>;

@compute @workgroup_size(256)
fn reduce_max(@builtin(local_invocation_id) local_id: vec3u, @builtin(global_id) global_id: vec3u) {
    let tid: u32 = local_id.x;
    let i: u32 = global_id.x;
    let n: u32 = arrayLength(&input.data);

    if (i < n) {
        shared_max_buf[tid] = input.data[i];
    } else {
        shared_max_buf[tid] = -3.402823e+38;
    }
    workgroupBarrier();

    var stride: u32 = 128u;
    while (stride > 0u) {
        if (tid < stride) {
            shared_max_buf[tid] = max(shared_max_buf[tid], shared_max_buf[tid + stride]);
        }
        workgroupBarrier();
        stride = stride >> 1u;
    }

    if (tid == 0u) {
        result.value = shared_max_buf[0];
    }
}

struct ScanInput {
    data: array<f32>,
}

struct ScanOutput {
    data: array<f32>,
}

@group(0) @binding(0) var<storage, read> scan_input: ScanInput;
@group(0) @binding(1) var<storage, read_write> scan_output: ScanOutput;

var<workgroup> scan_temp: array<f32, 512>;

@compute @workgroup_size(256)
fn blelloch_scan(@builtin(local_invocation_id) local_id: vec3u, @builtin(global_id) global_id: vec3u) {
    let tid: u32 = local_id.x;
    let i: u32 = global_id.x;
    let n: u32 = arrayLength(&scan_input.data);

    var ai: u32 = tid;
    var bi: u32 = tid + 256u;
    if (ai < n) {
        scan_temp[ai] = scan_input.data[ai];
    } else {
        scan_temp[ai] = 0.0;
    }
    if (bi < n) {
        scan_temp[bi] = scan_input.data[bi];
    } else {
        scan_temp[bi] = 0.0;
    }
    workgroupBarrier();

    var offset: u32 = 1u;
    var d: u32 = 128u;
    while (d > 0u) {
        if (tid < d) {
            let a: u32 = offset * (2u * tid + 1u) - 1u;
            let b: u32 = offset * (2u * tid + 2u) - 1u;
            scan_temp[b] = scan_temp[b] + scan_temp[a];
        }
        offset = offset * 2u;
        workgroupBarrier();
        d = d >> 1u;
    }

    if (tid == 0u) {
        scan_temp[511u] = 0.0;
    }
    workgroupBarrier();

    d = 1u;
    while (d < 256u) {
        offset = offset >> 1u;
        if (tid < d) {
            let a: u32 = offset * (2u * tid + 1u) - 1u;
            let b: u32 = offset * (2u * tid + 2u) - 1u;
            let t: f32 = scan_temp[a];
            scan_temp[a] = scan_temp[b];
            scan_temp[b] = scan_temp[b] + t;
        }
        workgroupBarrier();
        d = d * 2u;
    }

    if (ai < n) {
        scan_output.data[ai] = scan_temp[ai];
    }
    if (bi < n) {
        scan_output.data[bi] = scan_temp[bi];
    }
}