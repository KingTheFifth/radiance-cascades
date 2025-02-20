#version 450

in vec4 position;
in vec2 v_tex_coord;
out vec2 tex_coord;
out vec4 albedo;
out vec4 emissive;
uniform mat4 model_to_world;
uniform mat4 world_to_view;
uniform mat4 projection;
//in mat4 model_to_world; // This placement is necessary for the code to work

void main() {
    gl_Position = projection * world_to_view * model_to_world * vec4(position.xyz, 1.0);
    tex_coord = v_tex_coord;
    // albedo = s.albedo;
    // emissive = s.emissive;
}