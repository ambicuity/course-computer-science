# GLSL vs WGSL — Side-by-Side Shader Cheatsheet

A quick-reference for translating shaders between OpenGL (GLSL) and WebGPU (WGSL).

## Entry Points

| Concept          | GLSL (`#version 330 core`)         | WGSL                                     |
|------------------|-------------------------------------|------------------------------------------|
| Vertex shader    | `void main() { ... }`              | `@vertex fn vs(input) -> Output { ... }`|
| Fragment shader  | `void main() { ... }`              | `@fragment fn fs(input) -> Output { ... }`|
| Separate files   | Yes (two .glsl files)              | No (single .wgsl file, two entry points)|

## Scalar & Vector Types

| GLSL Type  | WGSL Type   | Meaning                       |
|------------|-------------|-------------------------------|
| `float`    | `f32`       | 32-bit float                  |
| `int`      | `i32`       | 32-bit signed integer         |
| `uint`     | `u32`       | 32-bit unsigned integer       |
| `bool`     | `bool`       | boolean                       |
| `vec2`     | `vec2f`     | 2-component float vector      |
| `vec3`     | `vec3f`     | 3-component float vector      |
| `vec4`     | `vec4f`     | 4-component float vector      |
| `ivec2`    | `vec2i`     | 2-component int vector        |
| `ivec3`    | `vec3i`     | 3-component int vector        |

## Matrix Types

| GLSL Type  | WGSL Type     | Meaning                  |
|------------|---------------|--------------------------|
| `mat2`     | `mat2x2f`     | 2×2 float matrix         |
| `mat3`     | `mat3x3f`     | 3×3 float matrix         |
| `mat4`     | `mat4x4f`     | 4×4 float matrix         |
| `mat2x3`   | `mat2x3f`     | 2 columns × 3 rows      |

## Vertex Input (Attributes)

| Concept       | GLSL                                    | WGSL                                          |
|---------------|------------------------------------------|-----------------------------------------------|
| Qualifier     | `layout(location=0) in vec3 a_pos;`    | `@location(0) pos: vec3f`                    |
| Binding       | Explicit location index                 | Explicit location index                        |
| Built-in pos  | `gl_Position` (gl_Position = ...)       | `@builtin(position) clip_pos: vec4f`         |
| Index         | `gl_VertexID`                           | `@builtin(vertex_index) vid: u32`            |

## Fragment Input (Varyings)

| Concept       | GLSL                           | WGSL                                    |
|---------------|---------------------------------|-----------------------------------------|
| Vertex out    | `out vec3 v_normal;`           | `@location(1) v_normal: vec3f` in struct|
| Fragment in   | `in vec3 v_normal;`             | `@location(1) v_normal: vec3f` in struct|
| Interpolation | Default: perspective-correct   | Default: perspective-correct            |
| Flat interp   | `flat out int v_id;`           | `@location(2) @interpolate(flat) id: i32`|

## Fragment Output

| Concept        | GLSL                                | WGSL                                    |
|----------------|--------------------------------------|-----------------------------------------|
| Single target  | `out vec4 frag_color;`              | `@location(0) frag_color: vec4f`       |
| Multiple RTs   | `layout(location=1) out vec4 g1;`  | `@location(1) g1: vec4f`               |
| Depth output   | `gl_FragDepth = ...`               | `@builtin(frag_depth) depth: f32`      |

## Uniforms

| Concept         | GLSL                              | WGSL                                       |
|-----------------|------------------------------------|---------------------------------------------|
| Single uniform  | `uniform mat4 u_mvp;`            | `@group(0) @binding(0) var<uniform> u_mvp: mat4x4f` |
| Uniform block   | `layout(std140) uniform Block`  | `struct U { ... }; @group(0) @binding(0) var<uniform> u: U` |
| Sampler         | `uniform sampler2D tex;`         | `@group(0) @binding(1) var tex: sampler_2d<f32>` |
| Storage buffer  | N/A (SSBO: `buffer`)             | `@group(0) @binding(2) var<storage, read> data: array<f32>` |

## Built-in Functions

| Function     | GLSL                    | WGSL                      | Notes                        |
|-------------|--------------------------|---------------------------|------------------------------|
| Normalize   | `normalize(v)`           | `normalize(v)`            | Unit vector                  |
| Dot product | `dot(a, b)`             | `dot(a, b)`               | Scalar product               |
| Cross prod  | `cross(a, b)`           | `cross(a, b)`             | Vector product               |
| Reflect     | `reflect(I, N)`         | `reflect(I, N)`           | Reflection vector            |
| Refract     | `refract(I, N, eta)`    | `refract(I, N, eta)`      | Refraction vector            |
| Mix/Lerp    | `mix(a, b, t)`          | `mix(a, b, t)`            | Linear interpolation          |
| Clamp       | `clamp(x, lo, hi)`     | `clamp(x, lo, hi)`        | Clamp to range                |
| Step        | `step(edge, x)`         | `step(edge, x)`           | 0 if x < edge, else 1        |
| Smoothstep  | `smoothstep(e0,e1,x)`   | `smoothstep(e0,e1,x)`     | Hermite interpolation        |
| Max / Min   | `max(a, b)`, `min(a,b)` | `max(a, b)`, `min(a,b)`   | Component-wise               |
| Pow         | `pow(x, y)`             | `pow(x, y)`               | Exponentiation                |
| Length      | `length(v)`             | `length(v)`               | Vector magnitude              |
| Distance    | `distance(a, b)`         | `distance(a, b)`          | Distance between two points   |
| Texture     | `texture(sampler, uv)`   | `textureLoad(tex, coord)` or `textureSample(tex, samp, uv)` | WGSL separates sampling from loading |

## Variable Declaration

| Concept     | GLSL                      | WGSL                               |
|-------------|---------------------------|-------------------------------------|
| Immutable   | `const float x = 1.0;`   | `const x: f32 = 1.0;` or `let x = 1.0;` |
| Mutable     | `float x = 1.0;`         | `var x: f32 = 1.0;`                |
| Inferred    | N/A                       | `let x = 1.0;` (inferred f32)     |

## Control Flow

| Concept     | GLSL                           | WGSL                                  |
|-------------|---------------------------------|---------------------------------------|
| If/else     | `if (x > 0) { ... } else { ... }` | `if x > 0 { ... } else { ... }`    |
| For loop    | `for (int i = 0; i < 10; i++)`   | `for (var i: i32 = 0; i < 10; i++)` |
| Switch      | `switch(x) { case 0: ... }`      | `switch x { case 0: ... }`          |
| Discard     | `discard;`                     | `discard;`                           |

## Common Patterns

### MVP Transform

**GLSL:**
```glsl
uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;

void main() {
    vec4 worldPos = u_model * vec4(a_position, 1.0);
    gl_Position = u_projection * u_view * worldPos;
}
```

**WGSL:**
```wgsl
struct Uniforms {
    model: mat4x4f,
    view: mat4x4f,
    projection: mat4x4f,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    let worldPos = u.model * vec4f(input.position, 1.0);
    output.clip_position = u.projection * u.view * worldPos;
    return output;
}
```

### Diffuse Lighting

**GLSL:**
```glsl
vec3 N = normalize(v_normal);
vec3 L = normalize(u_lightPos - v_worldPos);
float diff = max(dot(N, L), 0.0);
vec3 diffuse = u_lightColor * diff;
```

**WGSL:**
```wgsl
let N = normalize(input.v_normal);
let L = normalize(u.lightPos - input.worldPos);
let diff = max(dot(N, L), 0.0);
let diffuse = u.lightColor * diff;
```

### Texture Sampling

**GLSL:**
```glsl
uniform sampler2D u_texture;
in vec2 v_uv;

vec4 texColor = texture(u_texture, v_uv);
```

**WGSL:**
```wgsl
@group(0) @binding(1) var u_texture: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;

let texColor = textureSample(u_texture, u_sampler, input.v_uv);
```

## Quick Debugging Reference

| What to debug     | GLSL                                    | WGSL                                        |
|-------------------|------------------------------------------|---------------------------------------------|
| Normals           | `frag_color = vec4(v_normal * 0.5 + 0.5, 1.0);` | `return vec4f(input.v_normal * 0.5 + 0.5, 1.0);` |
| UVs               | `frag_color = vec4(v_uv, 0.0, 1.0);`   | `return vec4f(input.v_uv, 0.0, 1.0);`      |
| Depth             | `frag_color = vec4(vec3(gl_FragCoord.z), 1.0);` | `return vec4f(vec3f(input.clip_position.z / input.clip_position.w), 1.0);` |