#version 450

in vec3 position;
in vec2 v_tex_coord;
out vec2 tex_coord;

uniform mat4 world_to_view;
uniform mat4 projection;

void main() {
    gl_Position = vec4(position, 1.0);
    tex_coord = v_tex_coord;
}