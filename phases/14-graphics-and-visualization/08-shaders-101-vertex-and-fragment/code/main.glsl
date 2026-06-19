#version 330 core

// ─── Uniforms (constant per draw call) ───────────────────────────────────────
// Set by the CPU before each draw call.

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform vec3 u_lightPos;
uniform vec3 u_lightColor;
uniform vec3 u_ambientColor;
uniform float u_ambientStrength;
uniform vec3 u_viewPos;

// ─── Vertex Shader ──────────────────────────────────────────────────────────
// Runs once per vertex. Transforms position to clip space and passes
// world-space data to the fragment shader via varyings.

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_uv;

out vec3 v_worldPos;
out vec3 v_normal;
out vec2 v_uv;

void main()
{
    vec4 worldPos = u_model * vec4(a_position, 1.0);
    v_worldPos = worldPos.xyz;
    v_normal = mat3(transpose(inverse(u_model))) * a_normal;
    v_uv = a_uv;

    gl_Position = u_projection * u_view * worldPos;
}

// ─── Fragment Shader ─────────────────────────────────────────────────────────
// Runs once per pixel-fragment. Computes ambient + diffuse (Lambertian) lighting.
//
// To use this in a real OpenGL application, the vertex shader and fragment
// shader must be compiled as separate shader objects and linked into one
// program. They are shown here in one file for reference.
//
// Lighting model:
//   ambient  = u_ambientStrength * u_ambientColor
//   diffuse  = u_lightColor * max(dot(N, L), 0.0)
//   result   = (ambient + diffuse) * objectColor

in vec3 v_worldPos;
in vec3 v_normal;
in vec2 v_uv;

layout(location = 0) out vec4 frag_color;

void main()
{
    vec3 N = normalize(v_normal);
    vec3 L = normalize(u_lightPos - v_worldPos);
    vec3 V = normalize(u_viewPos - v_worldPos);

    // Ambient term: prevents completely black shadows
    vec3 ambient = u_ambientStrength * u_ambientColor;

    // Diffuse term: Lambert's cosine law
    float NdotL = max(dot(N, L), 0.0);
    vec3 diffuse = u_lightColor * NdotL;

    // Combine: object color is white (1,1,1) — multiply by a texture or
    // material color in a real application
    vec3 objectColor = vec3(1.0);
    vec3 result = (ambient + diffuse) * objectColor;

    frag_color = vec4(result, 1.0);
}