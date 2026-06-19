// ─── Struct Definitions ──────────────────────────────────────────────────────
// WGSL uses structs for all shader interface data.

struct Uniforms {
    model: mat4x4f,
    view: mat4x4f,
    projection: mat4x4f,
    lightPos: vec3f,
    lightColor: vec3f,
    ambientColor: vec3f,
    ambientStrength: f32,
    viewPos: vec3f,
    _pad1: f32,
};

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) worldPos: vec3f,
    @location(1) v_normal: vec3f,
    @location(2) v_uv: vec2f,
};

struct FragmentOutput {
    @location(0) frag_color: vec4f,
};

// ─── Uniform Bindings ────────────────────────────────────────────────────────

@group(0) @binding(0) var<uniform> u: Uniforms;

// ─── Vertex Shader ──────────────────────────────────────────────────────────
// Runs once per vertex. Transforms position to clip space and passes
// world-space data to the fragment shader via the VertexOutput struct.
// The normal matrix is computed as transpose(inverse(model)).

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    let worldPos: vec4f = u.model * vec4f(input.position, 1.0);
    output.worldPos = worldPos.xyz;

    // Normal matrix: transpose(inverse(model)) — for uniform scale,
    // mat3(model) suffices. For non-uniform scale, use the proper inverse.
    // Here we use mat3x3f(u.model) as a simplification for uniform scale.
    let normalMatrix: mat3x3f = mat3x3f(
        u.model[0].xyz,
        u.model[1].xyz,
        u.model[2].xyz,
    );
    output.v_normal = normalize(normalMatrix * input.normal);
    output.v_uv = input.uv;

    output.clip_position = u.projection * u.view * worldPos;

    return output;
}

// ─── Fragment Shader ─────────────────────────────────────────────────────────
// Runs once per pixel-fragment. Computes ambient + diffuse (Lambertian) lighting.
//
// Lighting model:
//   ambient  = u.ambientStrength * u.ambientColor
//   diffuse  = u.lightColor * max(dot(N, L), 0.0)
//   result   = (ambient + diffuse) * objectColor

@fragment
fn fs(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;

    let N: vec3f = normalize(input.v_normal);
    let L: vec3f = normalize(u.lightPos - input.worldPos);
    let V: vec3f = normalize(u.viewPos - input.worldPos);

    // Ambient term: prevents completely black shadows
    let ambient: vec3f = u.ambientStrength * u.ambientColor;

    // Diffuse term: Lambert's cosine law
    let NdotL: f32 = max(dot(N, L), 0.0);
    let diffuse: vec3f = u.lightColor * NdotL;

    // Combine: object color is white (1,1,1) — multiply by a texture
    // or material color in a real application
    let objectColor: vec3f = vec3f(1.0);
    let result: vec3f = (ambient + diffuse) * objectColor;

    output.frag_color = vec4f(result, 1.0);

    return output;
}