# Shaders 101 — Output Artifact

## shader_cheatsheet.md

A side-by-side GLSL vs WGSL syntax reference covering:

- **Types**: `vec2/3/4`, `mat2/3/4` (GLSL) vs `vec2f/3f/4f`, `mat2x2f/3x3f/4x4f` (WGSL)
- **Qualifiers**: `in`, `out`, `uniform` (GLSL) vs `@location(N)`, `@builtin(position)`, `@group(G) @binding(B)` (WGSL)
- **Entry points**: `void main()` (GLSL) vs `@vertex fn vs()`, `@fragment fn fs()` (WGSL)
- **Built-in functions**: `normalize`, `dot`, `max`, `clamp`, `mix`, `reflect`, `texture`
- **Variable passing**: varyings (`out`/`in` in GLSL) vs structs with `@location` fields (WGSL)

This cheatsheet is designed to be printed or kept open side-by-side while writing or porting shaders between OpenGL and WebGPU.