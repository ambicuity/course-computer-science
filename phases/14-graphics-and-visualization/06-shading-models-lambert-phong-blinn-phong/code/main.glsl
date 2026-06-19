#version 330 core

#define SHADING_MODEL 0  // 0 = Lambert, 1 = Phong, 2 = Blinn-Phong

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_uv;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform mat3 u_normal_matrix;

out vec3 v_position;
out vec3 v_normal;
out vec2 v_uv;

void main() {
    vec4 world_pos = u_model * vec4(a_position, 1.0);
    v_position = world_pos.xyz;
    v_normal = normalize(u_normal_matrix * a_normal);
    v_uv = a_uv;
    gl_Position = u_projection * u_view * world_pos;
}

--- FRAGMENT ---

#version 330 core

#define SHADING_MODEL 0  // 0 = Lambert, 1 = Phong, 2 = Blinn-Phong

in vec3 v_position;
in vec3 v_normal;
in vec2 v_uv;

uniform vec3 u_light_dir;
uniform vec3 u_light_color;
uniform vec3 u_ambient_color;
uniform vec3 u_diffuse_color;
uniform vec3 u_specular_color;
uniform float u_shininess;
uniform vec3 u_view_pos;

out vec4 frag_color;

const float PI = 3.14159265359;

vec3 lambert_shading(vec3 N, vec3 L, vec3 diffuse) {
    float NdotL = max(dot(N, L), 0.0);
    return diffuse * u_light_color * NdotL / PI;
}

vec3 phong_reflect_model(vec3 N, vec3 L, vec3 V, vec3 diffuse, vec3 specular, float shininess) {
    float NdotL = max(dot(N, L), 0.0);
    vec3 ambient = u_ambient_color * diffuse;
    vec3 diff = diffuse * u_light_color * NdotL;
    vec3 R = reflect(-L, N);
    float RdotV = max(dot(R, V), 0.0);
    float spec_pow = pow(RdotV, shininess);
    vec3 spec = specular * u_light_color * spec_pow;
    return ambient + diff + spec;
}

vec3 blinn_phong_model(vec3 N, vec3 L, vec3 V, vec3 diffuse, vec3 specular, float shininess) {
    float NdotL = max(dot(N, L), 0.0);
    vec3 ambient = u_ambient_color * diffuse;
    vec3 diff = diffuse * u_light_color * NdotL;
    vec3 H = normalize(L + V);
    float NdotH = max(dot(N, H), 0.0);
    float spec_pow = pow(NdotH, shininess);
    vec3 spec = specular * u_light_color * spec_pow;
    return ambient + diff + spec;
}

void main() {
    vec3 N = normalize(v_normal);
    vec3 L = normalize(u_light_dir);
    vec3 V = normalize(u_view_pos - v_position);

    vec3 result;
    if (SHADING_MODEL == 0) {
        result = lambert_shading(N, L, u_diffuse_color);
    } else if (SHADING_MODEL == 1) {
        result = phong_reflect_model(N, L, V, u_diffuse_color, u_specular_color, u_shininess);
    } else {
        result = blinn_phong_model(N, L, V, u_diffuse_color, u_specular_color, u_shininess);
    }

    frag_color = vec4(result, 1.0);
}