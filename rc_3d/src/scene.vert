#version 450

in vec4 position;
in vec2 v_tex_coord;
in vec3 v_normal;
out vec2 tex_coord;
out vec4 albedo;
out vec4 emissive;
out vec3 normal;
uniform vec4 v_albedo;
uniform mat4 model_to_world;
uniform mat4 world_to_view;
uniform mat4 projection;

void main() {
    gl_Position = projection * world_to_view * model_to_world * vec4(position.xyz, 1.0);
    tex_coord = v_tex_coord;
    albedo = v_albedo;
    emissive = vec4(0.0, 1.0, 0.0, 1.0);
    normal = mat3(world_to_view * model_to_world) * v_normal;
}