#version 450

flat in vec3 voxel_pos;
out vec4 color;

layout(binding = 0, rgba16f) uniform readonly image3D voxel_tex;

void main() {
    color = imageLoad(voxel_tex, ivec3(voxel_pos));
    if (color.a < 0.01) {
        discard;
    }
}