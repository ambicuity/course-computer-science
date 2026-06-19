# Shaders 101 — Vertex and Fragment

> A shader is a program that runs on the GPU — once per vertex, once per pixel. Mastering shaders means controlling everything you see on screen, from vertex placement to final color.

**Type:** Learn
**Languages:** GLSL, WGSL
**Prerequisites:** Phase 14 lessons 01–07
**Time:** ~75 minutes

## Learning Objectives

- Explain what a shader is and why it runs on the GPU instead of the CPU.
- Distinguish vertex shaders from fragment shaders and name their responsibilities.
- Trace data flow through the GPU pipeline: attributes → vertex shader → varyings → rasterizer → fragment shader → output.
- Read and write syntactically valid GLSL (#version 330 core) and WGSL shader code.
- Use uniforms, attributes, and varyings correctly to build a diffuse + ambient lighting shader.
- Debug shaders using visual debugging techniques (color mapping, step validation).

## The Problem

Every pixel on your screen was colored by a program. On a 1920×1080 display that is over two million pixels, redrawn 60 times per second. Your CPU cannot individually set 124 million pixels per second — that is the GPU's job. The GPU achieves this through **massive parallelism**: thousands of tiny cores each run the same small program on different data.

That small program is a **shader**. Without understanding shaders, you cannot:

- Control where vertices land on screen (objects appear in the wrong place).
- Control what color pixels become (objects look flat, wrong, or invisible).
- Debug rendering problems (black screens, flickering, distorted geometry).
- Move beyond fixed-function pipelines into real-time graphics.

This lesson builds the mental model of the GPU pipeline and the two shader stages you will use most: **vertex** and **fragment**.

## The GPU Pipeline

Before writing shaders, you need to understand the pipeline they live in:

```
  CPU sends data
        │
        ▼
 ┌──────────────────┐
 │  Vertex Puller    │  ← reads vertex buffers (attributes)
 └────────┬─────────┘
          │
          ▼
 ┌──────────────────┐
 │  Vertex Shader   │  ← YOUR CODE: runs once per vertex
 │  (per-vertex)    │     transforms position, passes data along
 └────────┬─────────┘
          │
          ▼
 ┌──────────────────┐
 │  Primitive       │  ← assembles triangles from vertices
 │  Assembly        │
 └────────┬─────────┘
          │
          ▼
 ┌──────────────────┐
 │  Rasterizer      │  ← determines which pixels a triangle covers
 │  (hardware)      │     interpolates varyings across the triangle
 └────────┬─────────┘
          │
          ▼
 ┌──────────────────┐
 │  Fragment Shader  │  ← YOUR CODE: runs once per pixel-fragment
 │  (per-pixel)     │     computes the final color
 └────────┬─────────┘
          │
          ▼
 ┌──────────────────┐
 │  Output Merger   │  ← depth test, stencil test, blend
 └──────────────────┘
          │
          ▼
      Framebuffer (screen)
```

The key insight: you write **two** programs. The GPU runs the vertex program N times (once per vertex) and the fragment program M times (once per pixel covered by a triangle). The hardware handles everything in between.

## Attributes, Uniforms, and Varyings

These three qualifiers control how data flows through the pipeline:

```
  Attribute (in)          Uniform               Varying (out → in)
  ──────────────          ───────               ──────────────────────
  Per-vertex data         Per-draw-call data    Interpolated per-pixel
  from vertex buffer      set once by CPU       passed vertex → fragment

  position                model matrix          world-space position
  normal                  view matrix           surface normal
  texcoord                projection matrix    texture coordinates
  color                   light position        vertex color
                          light color
```

### Attributes (Vertex Inputs)

These come from vertex buffers. Every vertex gets its own values:

```
  Vertex 0: position=(-1, -1, 0)  normal=(0,0,1)  uv=(0,0)
  Vertex 1: position=( 1, -1, 0)  normal=(0,0,1)  uv=(1,0)
  Vertex 2: position=( 0,  1, 0)  normal=(0,0,1)  uv=(0.5,1)
```

### Uniforms (Global Constants)

These are constant across all vertices and fragments in a single draw call. You set them from the CPU before drawing:

```
  u_model      = rotateY(45°)
  u_view       = lookAt(eye, center, up)
  u_projection = perspective(fov, aspect, near, far)
  u_lightPos   = (5, 5, 5)
  u_lightColor = (1, 1, 1)
```

### Varyings (Vertex → Fragment)

The vertex shader **writes** varyings. The rasterizer **interpolates** them. The fragment shader **reads** them. This interpolation is the magic that makes smooth lighting and textures possible:

```
  If vertex 0 outputs v_normal = (0, 0, 1)
  and vertex 2 outputs v_normal = (0, 1, 0)
  then a pixel exactly halfway between them receives:
  v_normal = (0, 0.5, 0.5)  ← automatically normalized perspective
```

**Barycentric interpolation**: for any point P inside a triangle with vertices A, B, C, compute weights (wA, wB, wC) such that `P = wA*A + wB*B + wC*C`. Apply those same weights to varyings: `varying_at_P = wA*vA + wB*vB + wC*vC`.

## Vertex Shader

The vertex shader's job: transform each vertex from model space to clip space and pass data to the fragment stage.

### The MVP Transform

```
  clip_position = projection × view × model × local_position

  Step 1: model      × local_position  = world position
  Step 2: view       × world_position   = view (camera) position
  Step 3: projection × view_position     = clip position

  In GLSL:  gl_Position = u_projection * u_view * u_model * vec4(a_position, 1.0);
```

Each matrix does one thing:

| Matrix       | From → To          | What it does                           |
|-------------|---------------------|----------------------------------------|
| Model       | object → world     | places the object in the scene         |
| View        | world → camera     | positions the camera                   |
| Projection  | camera → clip      | warps for perspective (near=far=small)  |

### Worked Example

A quad facing the camera. Two triangles, four vertices:

```
  Vertex positions (local space):       UVs:
  (-1, -1, 0)  ← bottom-left          (0, 0)
  ( 1, -1, 0)  ← bottom-right          (1, 0)
  ( 1,  1, 0)  ← top-right             (1, 1)
  (-1,  1, 0)  ← top-left              (0, 1)

  After MVP (assuming identity model, simple view/projection):
  Screen positions map roughly 1:1 to clip space.
  Each vertex's normal = (0, 0, 1) pointing toward camera.
```

## Fragment Shader

The fragment shader's job: compute the color of each pixel.

### Diffuse + Ambient Lighting

The simplest meaningful lighting model:

```
  ambient  = kA * u_lightColor * u_ambientStrength
  diffuse  = kD * u_lightColor * max(dot(N, L), 0)

  final_color = ambient + diffuse
```

Where:
- `N` = surface normal (interpolated from vertex shader)
- `L` = direction from surface to light (computed from light position)
- `kA`, `kD` = material reflectance (often just the surface color)
- `max(dot(N, L), 0)` = Lambert's cosine law: surfaces facing the light are brighter

```
  Light hitting surface head-on:  dot = 1.0  →  full brightness
  Light hitting at 45°:           dot = 0.7  →  70% brightness
  Light hitting at grazing angle: dot ≈ 0.0  →  darkness
  Light from behind:              dot < 0    →  clamped to 0 (darkness)
```

## Build It

### Step 1: Minimal GLSL Shader

A shader that just transforms vertices and outputs solid white:

```glsl
#version 330 core

uniform mat4 u_mvp;

layout(location = 0) in vec3 a_position;

void main() {
    gl_Position = u_mvp * vec4(a_position, 1.0);
}
```

```glsl
#version 330 core

layout(location = 0) out vec4 frag_color;

void main() {
    frag_color = vec4(1.0);
}
```

This gets something on screen, but it is flat white — no lighting, no texture, no depth cues.

### Step 2: Realistic GLSL Shader

Now we add normals, lighting, and varyings (see `code/main.glsl` for the full implementation):

```glsl
// Vertex shader passes world-space normal and position as varyings.
// Fragment shader computes ambient + diffuse lighting per pixel.
// This produces the lit surface you expect from a real-time 3D object.
```

Key differences from the minimal version:

1. **Varyings** carry normal and world position from vertex to fragment stage.
2. **Multiple uniforms** for model, view, projection, light properties.
3. **Normalization** of both the normal and light direction vectors.
4. **Clamped dot product** prevents negative lighting (light from behind).
5. **Ambient term** ensures no surface is completely black.

### Step 3: WGSL Equivalent

WGSL (WebGPU Shading Language) has different syntax but identical logic (see `code/main.wgsl`):

| Concept             | GLSL                        | WGSL                            |
|---------------------|-----------------------------|---------------------------------|
| Entry point         | `void main()`               | `@vertex fn vs(...)`           |
| Vertex output       | `gl_Position`               | `@builtin(position)`           |
| Varying             | `out vec3 v_normal`         | `@location(0) v_normal: vec3f` |
| Uniform binding     | `uniform mat4 u_mvp;`       | `@group(0) @binding(0)`       |
| Fragment output     | `out vec4 frag_color`       | `@location(0) vec4f`           |
| Types               | `vec3`, `mat4`              | `vec3f`, `mat4x4f`             |

## Debugging Shaders

Shaders are notoriously difficult to debug because they run on the GPU. You cannot `printf()` from a fragment shader. Here are the standard techniques:

### 1. Visual Debugging (Color Mapping)

Encode data as colors to see what is happening:

```
  Problem: "Is my normal correct?"
  Debug:   frag_color = vec4(v_normal * 0.5 + 0.5, 1.0);
  Why:     Normals range from (-1,-1,-1) to (1,1,1).
           Mapping to (0,0,0)-(1,1,1) makes them visible.
           +X = red, +Y = green, +Z = blue on screen.

  Problem: "Is UV mapping correct?"
  Debug:   frag_color = vec4(v_uv, 0.0, 1.0);
  Why:     UVs (0,0)-(1,1) become a red-green gradient.

  Problem: "Are my vertices transformed correctly?"
  Debug:   frag_color = vec4(v_worldPos * 0.1, 1.0);
  Why:     Scales world position so nearby objects show as color.
```

### 2. Step-by-Step Validation

Isolate each term and check it independently:

```
  Step 1: Output just ambient.      → Should be a dim but uniform color.
  Step 2: Output just diffuse.       → Should be darker on unlit sides.
  Step 3: Output the dot product.    → Should be bright where facing light.
  Step 4: Combine ambient + diffuse. → Final result.
```

### 3. Common Errors

| Symptom                   | Likely Cause                                    |
|---------------------------|-------------------------------------------------|
| Black screen              | Normal not normalized, or dot product always ≤0 |
| Inverted lighting         | Light direction pointing wrong way (L vs -L)    |
| Object appears at wrong scale | MVP matrices in wrong order or wrong type   |
| No depth sorting          | Depth test not enabled (not a shader bug)       |
| Faceted appearance        | Normals not interpolated (flat shading)          |

## Shader Variants: Uber-Shader vs Separate Programs

As you add features (texturing, normal mapping, shadows, skinning), you face a design choice:

**Option A: Uber-shader** — One shader with `#ifdef` switches:
```glsl
#ifdef USE_TEXTURE
    color *= texture(u_diffuseMap, v_uv);
#endif
#ifdef USE_NORMAL_MAP
    vec3 N = perturbNormal(v_normal, v_tangent, texture(u_normalMap, v_uv));
#endif
```
Pros: single compile unit, easy to add features. Cons: compilation gets slow, conditional branching on GPU.

**Option B: Separate shader programs** — One program per feature combination:
```
  shader_basic       → MVP + ambient + diffuse
  shader_textured    → MVP + ambient + diffuse + texture
  shader_normal_map  → MVP + ambient + diffuse + texture + normal map
```
Pros: each program is lean and fast. Cons: combinatorial explosion as features multiply.

In practice, most engines use a hybrid: a ubershader with permutations that are compiled into separate programs offline.

## Read the Source

- **The Book of Shaders** — https://thebookofshaders.com/ — Interactive fragment shader tutorial.
- **WebGPU WGSL Spec** — https://www.w3.org/TR/WGSL/ — Authoritative WGSL language reference.
- **OpenGL Wiki: Vertex Specification** — https://www.khronos.org/opengl/wiki/Vertex_Specification — How attributes feed the pipeline.
- **Filament Materials Guide** — https://google.github.io/filament/Filament.html — Production PBR lighting math.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`shader_cheatsheet.md`** — A side-by-side GLSL vs WGSL syntax reference covering types, qualifiers, built-in functions, entry points, and variable passing.

## Exercises

1. **Easy** — Modify the fragment shader to add specular highlights (Phong model: `pow(max(dot(R, V), 0), shininess)` where R is the reflected light direction and V is the view direction).
2. **Medium** — Add a second light source (directional) to the shader. You will need a `u_light2Dir` uniform and add a diffuse term for the second light.
3. **Hard** — Implement a simple toon/cel shader: quantize the diffuse intensity into 3-4 discrete bands instead of smooth gradients. Use `floor(dot * bands) / bands` as the intensity.

## Key Terms

| Term                | What people say        | What it actually means                                              |
|---------------------|------------------------|---------------------------------------------------------------------|
| Shader              | "GPU code"             | A program compiled to run on GPU hardware, not CPU                  |
| Vertex shader       | "vertex program"       | Runs once per vertex; transforms position and passes data downstream|
| Fragment shader     | "pixel shader"         | Runs once per pixel-fragment; computes the final color             |
| Varying             | "interpolated output"  | Data written by the vertex shader, linearly interpolated, read by fragment |
| Uniform             | "constant"             | Data that stays the same for all vertices/fragments in a draw call |
| Attribute           | "per-vertex input"     | Data read from vertex buffers, different per vertex                 |
| Rasterizer          | "the hardware"         | Fixed-function stage that determines which pixels a triangle covers |
| MVP matrix          | "the transform"        | Model × View × Projection: converts local coords to clip coords     |
| GLSL                | "OpenGL shading lang"  | OpenGL / OpenGL ES shading language (C-like syntax)                |
| WGSL       | "WebGPU shading lang"  | WebGPU Shading Language (Rust-like syntax, for WebGPU)             |
| Barycentric coords  | "triangle weights"    | (wA, wB, wC) weights used to interpolate varyings across a triangle |

## Further Reading

- **The Book of Shaders** — Patricio Gonzalez Vivo and Jen Lowe — The best interactive introduction to fragment shaders.
- **Learn OpenGL** — https://learnopengl.com/ — Comprehensive OpenGL/GLSL tutorial series with code.
- **WebGPU Fundamentals** — https://webgpufundamentals.org/ — WebGPU and WGSL tutorials.
- **Real-Time Rendering, 4th Ed.** — Akenine-Möller et al. — Chapter on the graphics pipeline.
- **GPU Gems** — https://developer.nvidia.com/gpugems/ — Classic GPU programming articles.