#version 450

in vec2 tex_coord;
in vec4 albedo;
//in vec4 emissive;
in vec3 normal;
in vec3 tangent;
in vec3 bitangent;
layout(location = 0) out vec4 g_albedo;
layout(location = 1) out vec4 g_emissive;
layout(location = 2) out vec4 g_normal;

uniform vec3 emissive;
uniform int has_emissive;
uniform vec3 diffuse;
uniform int has_diffuse;
uniform vec3 specular;
uniform int has_specular;
uniform float opacity;
uniform int has_opacity;

uniform sampler2D diffuse_tex;
uniform int has_diffuse_tex;
uniform sampler2D specular_tex;
uniform int has_specular_tex;
uniform sampler2D opacity_tex;
uniform int has_opacity_tex;

uniform sampler2D normal_map;
uniform int has_normal_map;

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
    vec2 adjusted_tex_coord = vec2(tex_coord.x, 1.0 - tex_coord.y);

    if (has_opacity_tex == 1 && texture(opacity_tex, adjusted_tex_coord).r < 0.1) {
        discard;
    }

    if (has_diffuse_tex == 1) {
        g_albedo = texture(diffuse_tex, adjusted_tex_coord);
    }
    else if (has_diffuse == 1) {
        g_albedo = vec4(diffuse, 1.0);
    }
    else {
        g_albedo = albedo;
    }

    vec3 normal = normalize(normal);
    if (has_normal_map == 1) {
        vec3 normal_sample = texture(normal_map, adjusted_tex_coord).xyz;
        normal_sample = normal_sample * 2.0 - 1.0;

        normal = normal_sample.x * normalize(tangent) + normal_sample.y * normalize(bitangent) + normal_sample.z * normal;
        normal = normalize(normal);
    }

    g_emissive = vec4(emissive, 1.0);
    //g_emissive.a = albedo.a;
    g_normal = vec4(octahedral_encode(normalize(normal)), 0.0, 1.0);
}