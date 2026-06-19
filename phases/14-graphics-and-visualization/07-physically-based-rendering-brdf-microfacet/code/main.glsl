#version 330 core

in vec3 v_position;
in vec3 v_normal;
in vec2 v_texcoord;

out vec4 frag_color;

struct Material {
    vec3 albedo;
    float metallic;
    float roughness;
    float ao;
};

struct Light {
    vec3 position;
    vec3 color;
};

uniform Material u_material;
uniform Light u_lights[4];
uniform vec3 u_cam_pos;

const float PI = 3.14159265359;

float distribution_ggx(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float nom = a2;
    float denom = NdotH2 * (a2 - 1.0) + 1.0;
    denom = PI * denom * denom;

    return nom / max(denom, 0.0001);
}

float geometry_schlick_ggx(float NdotV, float roughness) {
    float r = roughness + 1.0;
    float k = (r * r) / 8.0;

    float nom = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return nom / max(denom, 0.0001);
}

float geometry_smith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2 = geometry_schlick_ggx(NdotV, roughness);
    float ggx1 = geometry_schlick_ggx(NdotL, roughness);

    return ggx1 * ggx2;
}

vec3 fresnel_schlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

vec3 fresnel_schlick_roughness(float cosTheta, vec3 F0, float roughness) {
    return F0 + (max(vec3(1.0 - roughness), F0) - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

void main() {
    vec3 N = normalize(v_normal);
    vec3 V = normalize(u_cam_pos - v_position);

    vec3 F0 = mix(vec3(0.04), u_material.albedo, u_material.metallic);

    vec3 Lo = vec3(0.0);

    for (int i = 0; i < 4; ++i) {
        vec3 L = normalize(u_lights[i].position - v_position);
        vec3 H = normalize(V + L);

        float distance = length(u_lights[i].position - v_position);
        float attenuation = 1.0 / (distance * distance);
        vec3 radiance = u_lights[i].color * attenuation;

        float NDF = distribution_ggx(N, H, u_material.roughness);
        float G = geometry_smith(N, V, L, u_material.roughness);
        vec3 F = fresnel_schlick(max(dot(H, V), 0.0), F0);

        float NdotL = max(dot(N, L), 0.0);

        vec3 numerator = NDF * G * F;
        float denominator = 4.0 * max(dot(N, V), 0.0) * max(NdotL, 0.001);
        vec3 specular = numerator / denominator;

        vec3 kS = F;
        vec3 kD = vec3(1.0) - kS;
        kD *= 1.0 - u_material.metallic;

        vec3 diffuse = kD * u_material.albedo / PI;

        Lo += (diffuse + specular) * radiance * NdotL;
    }

    vec3 ambient = vec3(0.03) * u_material.albedo * u_material.ao;
    vec3 color = ambient + Lo;

    color = color / (color + vec3(1.0));
    color = pow(color, vec3(1.0 / 2.2));

    frag_color = vec4(color, 1.0);
}