#version 450

in vec2 tex_coord;
in float texture_index;
in vec4 albedo;
in vec4 emissive;
uniform sampler2DArray tex_array;
layout(location = 0) out vec4 color;
layout(location = 1 )out vec4 g_emissive;

void main() {
    color = texture(tex_array, vec3(tex_coord, texture_index));
    g_emissive = emissive;
    g_emissive.a = color.a;
}