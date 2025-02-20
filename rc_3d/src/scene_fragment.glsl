#version 450

in vec2 tex_coord;
in vec4 albedo;
in vec4 emissive;
layout(location = 0) out vec4 color;
layout(location = 1 )out vec4 g_emissive;

void main() {
    color = vec4(1.0);
    g_emissive = emissive;
    g_emissive.a = color.a;
}