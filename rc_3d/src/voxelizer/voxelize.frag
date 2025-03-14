#version 450

in vec3 frag_world_pos;
in vec3 frag_normal;
in vec4 frag_albedo;
in float frag_axis;

layout(binding = 0, rgba16f) uniform writeonly image3D voxel_tex;

out vec4 color;

bool is_inside_cube(vec3 point, float half_side_length) {
    return all(lessThan(abs(point), vec3(half_side_length)));
}

void main() {
    if (!is_inside_cube(frag_world_pos, 1.0)) {
        discard;
    }

    vec3 voxel_pos = frag_world_pos;
    if (frag_axis == 1.0) {
        voxel_pos = frag_world_pos.zyx;
    }
    else if (frag_axis == 2.0) {
        voxel_pos = frag_world_pos.zxy;
    }

    voxel_pos = 0.5*voxel_pos+ 0.5;
    imageStore(voxel_tex, ivec3(voxel_pos * imageSize(voxel_tex)), frag_albedo);
    //imageStore(voxel_tex, ivec3(vec3(0.0, 0.0, 0.0) * imageSize(voxel_tex)), vec4(1.0));
    color = vec4(frag_albedo.rgb, 1.0);
}