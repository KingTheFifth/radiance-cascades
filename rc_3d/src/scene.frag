#version 450

in vec2 tex_coord;
in vec4 albedo;
in vec4 emissive;
in vec3 normal;
layout(location = 0) out vec4 g_albedo;
layout(location = 1) out vec4 g_emissive;
layout(location = 2) out vec4 g_normal;

void main() {
    g_albedo = albedo;
    g_emissive = emissive;
    g_emissive.a = albedo.a;
    g_normal = vec4(normalize(normal), 1.0);
}