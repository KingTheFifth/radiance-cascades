#version 450

in vec2 tex_coord;
in vec4 albedo;
in vec4 emissive;
in vec3 normal;
layout(location = 0) out vec4 g_albedo;
layout(location = 1) out vec4 g_emissive;
layout(location = 2) out vec4 g_normal;

vec2 sign_not_zero(vec2 v) {
    return vec2(
        (v.x >= 0.0) ? 1.0 : -1.0,
        (v.y >= 0.0) ? 1.0 : -1.0
    );
}

vec2 octahedral_encode(vec3 v) {
    // Based on https://knarkowicz.wordpress.com/2014/04/16/octahedron-normal-vector-encoding/
    vec2 n = v.xy;
    n = n * (1.0 / (abs(v.x) + abs(v.y) + abs(v.z))); 
    n = (v.z < 0.0) ? ((vec2(1.0) - abs(n.yx)) * sign_not_zero(n)) : n.xy;
    //return n * 0.5 + 0.5;
    return n;
}

void main() {
    g_albedo = albedo;
    g_emissive = emissive;
    //g_emissive.a = albedo.a;
    g_normal = vec4(octahedral_encode(normalize(normal)), 0.0, 1.0);
}