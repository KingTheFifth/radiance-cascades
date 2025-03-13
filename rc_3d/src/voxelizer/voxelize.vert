#version 450

in vec3 position;
in vec3 normal;
in vec2 tex_coord;

uniform mat4 model_to_world;
uniform mat4 world_to_view;
uniform mat4 projection;

uniform vec4 albedo;

out vec3 geom_world_pos;
out vec3 geom_normal;
out vec4 geom_albedo;

void main() {
    geom_world_pos = (model_to_world * vec4(position, 1.0)).xyz;
    geom_normal = normalize(mat3(model_to_world) * normal);
    geom_albedo = albedo;
    gl_Position = projection * world_to_view * vec4(geom_world_pos, 1.0);
}
