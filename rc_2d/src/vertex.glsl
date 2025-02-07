#version 450

in vec4 position;
in vec2 tex_coord;
out vec2 v_tex_coord;

void main(void) {
    v_tex_coord = tex_coord;
   gl_Position = position; 
}