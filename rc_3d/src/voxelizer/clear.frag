#version 450

in vec2 tex_coord;

uniform vec4 clear_color;
layout(binding = 0, rgba16f) uniform writeonly image3D voxel_tex;
uniform ivec3 voxel_resolution;

void main() {
    ivec2 xy = ivec2(floor(tex_coord * voxel_resolution.xy));
    for (uint z = 0; z < voxel_resolution.z; z++) {
        imageStore(voxel_tex, ivec3(xy, z), clear_color);
    }
}