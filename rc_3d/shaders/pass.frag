#version 450

in vec2 tex_coord;
uniform sampler2D tex;
uniform int lod;
out vec4 color;

void main() {
    color = vec4(textureLod(tex, tex_coord, lod).rg, 0.0, 1.0);
}