#version 450

in vec2 v_tex_coord;
uniform sampler2D tex_unit;
out vec4 color;

const int dir_count = 16;
const int probe_count = 6;

void main(void) {
    ivec2 probe_index = ivec2(probe_count * v_tex_coord);
    //color = texture(tex_unit, v_tex_coord);
    color = vec4(vec2(probe_index) / probe_count, 0.0, 1.0);
}