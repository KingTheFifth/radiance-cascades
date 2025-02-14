#version 450

in vec4 position;
in vec2 v_tex_coord;
in float v_texture_index;
out vec2 tex_coord;
out float texture_index;
in mat4 model_to_world; // This placement is necessary for the code to work

void main() {
    gl_Position = model_to_world * vec4(position.xyz, 1.0);
    tex_coord = v_tex_coord;
    texture_index = v_texture_index;
}