#version 450

in vec3 frag_world_pos;
in vec3 frag_normal;
in vec4 frag_albedo;

layout(binding = 0, rgba16f) uniform writeonly image3D voxel_tex;

bool inside_unity_cube(vec3 point) {
    return all(lessThan(abs(point), vec3(1.0)));
}

void main() {
    if (!inside_unity_cube(frag_world_pos)) {
        discard;
    }

    vec3 voxel_pos = 0.5*frag_world_pos + 0.5;
    imageStore(voxel_tex, ivec3(voxel_pos * imageSize(voxel_tex)), frag_albedo);
}