#version 450

in vec4 position;
in vec2 v_tex_coord;
//in float v_texture_index;
out vec2 tex_coord;
out float texture_index;
out vec4 albedo;
out vec4 emissive;
//in mat4 model_to_world; // This placement is necessary for the code to work

struct Sprite {
    mat4 model_to_world;
    vec4 albedo;
    vec4 emissive;
    float texture_layer;
};

layout(std430) buffer SpriteBuffer {
    Sprite[] sprites;
};

void main() {
    Sprite s = sprites[gl_InstanceID];
    gl_Position = s.model_to_world * vec4(position.xyz, 1.0);
    tex_coord = v_tex_coord;
    texture_index = s.texture_layer;
    albedo = s.albedo;
    emissive = s.emissive;
}