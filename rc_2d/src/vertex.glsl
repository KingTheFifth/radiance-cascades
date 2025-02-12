#version 450

in vec4 position;
in vec2 v_tex_coord;
out vec2 tex_coord;

void main(void) {
    tex_coord = v_tex_coord;
    gl_Position = position; 
}