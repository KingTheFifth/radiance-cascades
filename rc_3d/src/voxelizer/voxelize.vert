#version 450

in vec3 position;
in vec3 normal;
in vec2 tex_coord;

uniform mat4 model_to_world;

uniform vec4 albedo;
uniform vec4 emissive;

out vec3 geom_world_pos;
out vec3 geom_normal;
out vec4 geom_albedo;
out vec4 geom_emissive;
out vec2 geom_tex_coord;

void main() {
    geom_world_pos = (model_to_world * vec4(position, 1.0)).xyz;
    geom_normal = normalize(mat3(model_to_world) * normal);
    geom_albedo = albedo;
    geom_emissive = emissive;
    geom_tex_coord = tex_coord;
    gl_Position = vec4(geom_world_pos, 1.0);
}
