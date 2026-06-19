#version 330 core

// ============================================================
// Real-Time Techniques — Deferred, Tiled, Cluster
// GLSL Shaders: Geometry Pass + Lighting Pass
// ============================================================

// ------------------------------------------------------------
// GEOMETRY PASS — Vertex Shader
// Writes world-space position and normal to G-buffer
// ------------------------------------------------------------

#ifdef GEOMETRY_PASS_VS

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_texcoord;
layout(location = 3) in vec3 a_albedo;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform mat3 u_normal_matrix;

out vec3 v_world_pos;
out vec3 v_normal;
out vec2 v_texcoord;
out vec3 v_albedo;

void main()
{
    vec4 world_pos = u_model * vec4(a_position, 1.0);
    v_world_pos = world_pos.xyz;
    v_normal = normalize(u_normal_matrix * a_normal);
    v_texcoord = a_texcoord;
    v_albedo = a_albedo;
    gl_Position = u_projection * u_view * world_pos;
}

#endif

// ------------------------------------------------------------
// GEOMETRY PASS — Fragment Shader
// Writes G-buffer into multiple render targets (MRT)
// ------------------------------------------------------------

#ifdef GEOMETRY_PASS_FS

layout(location = 0) out vec4 gbuf_albedo_spec;
layout(location = 1) out vec4 gbuf_normal_rough;
layout(location = 2) out vec4 gbuf_position;

in vec3 v_world_pos;
in vec3 v_normal;
in vec2 v_texcoord;
in vec3 v_albedo;

uniform float u_roughness;
uniform float u_specular;

void main()
{
    gbuf_albedo_spec = vec4(v_albedo, u_specular);
    gbuf_normal_rough = vec4(normalize(v_normal), u_roughness);
    gbuf_position = vec4(v_world_pos, 1.0);
}

#endif

// ------------------------------------------------------------
// LIGHTING PASS — Vertex Shader (full-screen triangle)
// ------------------------------------------------------------

#ifdef LIGHTING_PASS_VS

out vec2 v_texcoord;

void main()
{
    // Full-screen triangle trick: 3 vertices, no VAO needed
    // Vertex 0: (-1, -1) uv(0,0)
    // Vertex 1: ( 3, -1) uv(2,0)
    // Vertex 2: (-1,  3) uv(0,2)
    float x = float((gl_VertexID & 1) << 2) - 1.0;
    float y = float((gl_VertexID & 2) << 1) - 1.0;
    v_texcoord = vec2((x + 1.0) * 0.5, (y + 1.0) * 0.5);
    gl_Position = vec4(x, y, 0.0, 1.0);
}

#endif

// ------------------------------------------------------------
// LIGHTING PASS — Fragment Shader
// Reads G-buffer and computes per-pixel Blinn-Phong lighting
// ------------------------------------------------------------

#ifdef LIGHTING_PASS_FS

layout(location = 0) out vec4 frag_color;

in vec2 v_texcoord;

uniform sampler2D u_gbuf_albedo_spec;
uniform sampler2D u_gbuf_normal_rough;
uniform sampler2D u_gbuf_position;
uniform vec2 u_screen_size;
uniform vec3 u_camera_pos;

struct PointLight {
    vec3 position;
    vec3 color;
    float radius;
};

#define MAX_LIGHTS 256
uniform int u_num_lights;
uniform PointLight u_lights[MAX_LIGHTS];

vec3 compute_blinn_phong(vec3 pos, vec3 norm, vec3 albedo, float spec, float rough, vec3 view_dir)
{
    vec3 ambient = albedo * 0.05;
    vec3 result = ambient;

    for (int i = 0; i < u_num_lights; i++) {
        vec3 light_vec = u_lights[i].position - pos;
        float dist = length(light_vec);

        if (dist > u_lights[i].radius) {
            continue;
        }

        vec3 light_dir = light_vec / dist;

        // Attenuation: quadratic falloff
        float attenuation = 1.0 / (1.0 + 0.09 * dist + 0.032 * dist * dist);
        // Smooth cutoff at radius
        float cutoff = 1.0 - smoothstep(u_lights[i].radius * 0.8, u_lights[i].radius, dist);

        // Diffuse (Lambertian)
        float ndotl = max(dot(norm, light_dir), 0.0);
        vec3 diffuse = albedo * u_lights[i].color * ndotl;

        // Specular (Blinn-Phong)
        vec3 half_vec = normalize(light_dir + view_dir);
        float shininess = mix(256.0, 4.0, rough);
        float ndoth = max(dot(norm, half_vec), 0.0);
        float spec_intensity = pow(ndoth, shininess) * spec;
        vec3 specular = u_lights[i].color * spec_intensity;

        result += (diffuse + specular) * attenuation * cutoff;
    }

    return result;
}

void main()
{
    vec4 albedo_spec = texture(u_gbuf_albedo_spec, v_texcoord);
    vec4 normal_rough = texture(u_gbuf_normal_rough, v_texcoord);
    vec4 position    = texture(u_gbuf_position, v_texcoord);

    vec3 albedo = albedo_spec.rgb;
    float spec  = albedo_spec.a;
    vec3 norm   = normalize(normal_rough.rgb);
    float rough = normal_rough.a;
    vec3 pos    = position.rgb;

    // Skip background pixels (position == 0 means no geometry written)
    if (length(pos) < 0.001) {
        frag_color = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    vec3 view_dir = normalize(u_camera_pos - pos);
    vec3 color = compute_blinn_phong(pos, norm, albedo, spec, rough, view_dir);

    // HDR tone mapping (Reinhard) + gamma correction
    color = color / (color + vec3(1.0));
    color = pow(color, vec3(1.0 / 2.2));

    frag_color = vec4(color, 1.0);
}

#endif

// ------------------------------------------------------------
// TILED FORWARD+ — Light Culling Compute Shader
// Divides screen into 16x16 tiles, assigns lights per tile
// ------------------------------------------------------------

#ifdef TILED_CULL_CS

layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

#define TILE_SIZE 16

struct PointLightCS {
    vec3 position;
    vec3 color;
    float radius;
};

layout(std430, binding = 0) readonly buffer LightBuffer {
    PointLightCS lights[];
};

layout(std430, binding = 1) readonly buffer DepthBuffer {
    float depths[];
};

layout(std430, binding = 2) writeonly buffer TileLightCountBuffer {
    uint tile_light_counts[];
};

uniform int u_num_lights;
uniform int u_screen_width;
uniform int u_screen_height;
uniform mat4 u_view;
uniform mat4 u_inv_projection;

shared uint s_min_depth;
shared uint s_max_depth;
shared uint s_light_count;

void main()
{
    ivec2 tile_id = ivec2(gl_WorkGroupID.xy);
    ivec2 local_id = ivec2(gl_LocalInvocationID.xy);
    ivec2 pixel_coord = tile_id * TILE_SIZE + local_id;

    uint local_index = local_id.y * TILE_SIZE + local_id.x;

    // Initialize shared memory
    if (local_index == 0) {
        s_min_depth = 0xFFFFFFFF;
        s_max_depth = 0;
        s_light_count = 0;
    }
    barrier();

    // Read depth for this pixel
    if (pixel_coord.x < u_screen_width && pixel_coord.y < u_screen_height) {
        float d = depths[pixel_coord.y * u_screen_width + pixel_coord.x];
        uint d_bits = floatBitsToUint(d);
        atomicMin(s_min_depth, d_bits);
        atomicMax(s_max_depth, d_bits);
    }
    barrier();

    // Each thread checks one light against the tile
    uint light_idx = local_index;
    if (light_idx < uint(u_num_lights)) {
        // Project light into screen space and check tile overlap
        vec4 view_pos = u_view * vec4(lights[light_idx].position, 1.0);
        vec4 clip_pos = u_inv_projection * vec4(view_pos.xyz / max(view_pos.w, 0.001), 1.0);
        vec3 ndc = clip_pos.xyz / clip_pos.w;
        vec2 screen_pos = (ndc.xy + 1.0) * 0.5;
        ivec2 light_tile = ivec2(screen_pos * vec2(u_screen_width, u_screen_height)) / TILE_SIZE;

        // Check if light's tile range overlaps this tile
        float radius_tiles = lights[light_idx].radius / float(TILE_SIZE);
        ivec2 min_tile = max(light_tile - ivec2(int(radius_tiles)), ivec2(0));
        ivec2 max_tile = min(light_tile + ivec2(int(radius_tiles)),
                             ivec2(u_screen_width / TILE_SIZE, u_screen_height / TILE_SIZE));

        if (tile_id.x >= min_tile.x && tile_id.x <= max_tile.x &&
            tile_id.y >= min_tile.y && tile_id.y <= max_tile.y) {
            atomicAdd(s_light_count, 1);
        }
    }
    barrier();

    // Write tile light count
    if (local_index == 0) {
        uint tile_idx = tile_id.y * ((u_screen_width + TILE_SIZE - 1) / TILE_SIZE) + tile_id.x;
        tile_light_counts[tile_idx] = s_light_count;
    }
}

#endif

// ------------------------------------------------------------
// CLUSTERED — Light Assignment Compute Shader
// Extends tiling into depth (3D frustum clusters)
// ------------------------------------------------------------

#ifdef CLUSTERED_CULL_CS

layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

#define TILE_SIZE_X 16
#define TILE_SIZE_Y 16
#define NUM_DEPTH_SLICES 32

struct PointLightCL {
    vec3 position;
    vec3 color;
    float radius;
};

layout(std430, binding = 0) readonly buffer LightBufferCL {
    PointLightCL lights_cl[];
};

layout(std430, binding = 1) writeonly buffer ClusterLightCountBuffer {
    uint cluster_light_counts[];
};

uniform int u_num_lights;
uniform int u_screen_width;
uniform int u_screen_height;
uniform float u_near;
uniform float u_far;
uniform mat4 u_view;
uniform mat4 u_inv_projection;

// Exponential depth slice distribution: more slices near camera
float slice_depth(int slice)
{
    float z_near = u_near;
    float z_far = u_far;
    float s = float(slice) / float(NUM_DEPTH_SLICES);
    return z_near * pow(z_far / z_near, s);
}

void main()
{
    ivec3 cluster_id = ivec3(gl_WorkGroupID.xy, gl_GlobalInvocationID.z);
    ivec2 local_id = ivec2(gl_LocalInvocationID.xy);

    // Compute cluster depth range
    float z_near_slice = slice_depth(cluster_id.z);
    float z_far_slice = slice_depth(cluster_id.z + 1);

    // Compute cluster screen-space AABB
    float tile_size_x = float(TILE_SIZE_X) / float(u_screen_width) * 2.0;
    float tile_size_y = float(TILE_SIZE_Y) / float(u_screen_height) * 2.0;
    float min_x = -1.0 + float(cluster_id.x) * tile_size_x;
    float max_x = min_x + tile_size_x;
    float min_y = -1.0 + float(cluster_id.y) * tile_size_y;
    float max_y = min_y + tile_size_y;

    uint count = 0;

    for (int i = 0; i < u_num_lights; i++) {
        vec4 view_pos = u_view * vec4(lights_cl[i].position, 1.0);
        float light_z = -view_pos.z; // view-space depth (negative z is forward)
        float light_radius = lights_cl[i].radius;

        // Check depth overlap
        if (light_z + light_radius < z_near_slice || light_z - light_radius > z_far_slice) {
            continue;
        }

        // Check screen-space tile overlap
        vec4 clip_pos = u_inv_projection * view_pos;
        vec2 ndc = clip_pos.xy / clip_pos.w;

        if (ndc.x - light_radius > max_x || ndc.x + light_radius < min_x ||
            ndc.y - light_radius > max_y || ndc.y + light_radius < min_y) {
            continue;
        }

        count++;
    }

    uint cluster_linear_idx = cluster_id.z *
        ((u_screen_width + TILE_SIZE_X - 1) / TILE_SIZE_X) *
        ((u_screen_height + TILE_SIZE_Y - 1) / TILE_SIZE_Y) +
        cluster_id.y * ((u_screen_width + TILE_SIZE_X - 1) / TILE_SIZE_X) +
        cluster_id.x;

    if (local_id.x == 0 && local_id.y == 0) {
        cluster_light_counts[cluster_linear_idx] = count;
    }
}

#endif