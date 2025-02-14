#version 450

in vec2 tex_coord;
in float texture_index;
uniform sampler2DArray tex_array;
out vec4 color;

void main() {
    color = texture(tex_array, vec3(tex_coord, texture_index));
}