#version 450

in vec3 frag_world_pos;
in vec3 frag_normal;
in vec4 frag_albedo;

layout(binding = 0, rgba16f) uniform writeonly image3D voxel_tex;

out vec4 color;

bool is_inside_cube(vec3 point, float half_side_length) {
    return all(lessThan(abs(point), vec3(half_side_length)));
}

void main() {
    if (!is_inside_cube(frag_world_pos, 1.0)) {
        discard;
    }

    vec3 voxel_pos = 0.5*frag_world_pos + 0.5;
    imageStore(voxel_tex, ivec3(voxel_pos * imageSize(voxel_tex)), frag_albedo);
    color = vec4(frag_albedo.rgb, 1.0);
}