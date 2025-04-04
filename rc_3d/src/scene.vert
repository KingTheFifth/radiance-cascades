#version 450

in vec4 position;
in vec2 v_tex_coord;
in vec3 v_normal;
in vec3 v_tangent;
in vec3 v_bitangent;
out vec2 tex_coord;
out vec4 albedo;
//out vec4 emissive;
out vec3 normal;
out vec3 tangent;
out vec3 bitangent;
uniform vec4 v_albedo;
uniform vec4 v_emissive;
uniform mat4 model_to_world;
uniform mat4 world_to_view;
uniform mat4 projection;

void main() {
    gl_Position = projection * world_to_view * model_to_world * vec4(position.xyz, 1.0);
    tex_coord = v_tex_coord;
    albedo = v_albedo;
    //emissive = v_emissive;
    normal = mat3(model_to_world) * v_normal;
    tangent = mat3(model_to_world) * v_tangent;
    bitangent = mat3(model_to_world) * v_bitangent;
}