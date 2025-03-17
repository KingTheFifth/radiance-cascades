#version 450

in vec4 frag_world_pos;
in vec3 frag_normal;
in vec4 frag_albedo;
flat in int frag_axis;

layout(binding = 0, rgba16f) uniform writeonly image3D voxel_tex;

uniform ivec3 voxel_resolution;

out vec4 color;

bool is_inside_cube(vec3 point, float half_side_length) {
    return all(lessThan(abs(point), vec3(half_side_length)));
}

void main() {
    //if (!is_inside_cube(frag_world_pos, 1.0)) {
    //    discard;
    //}

    vec3 world_pos = vec3(frag_world_pos.xy, 1.0 - frag_world_pos.z) * voxel_resolution;
    ivec3 voxel_pos = ivec3(gl_FragCoord.xy, gl_FragCoord.z * voxel_resolution.z);
    if (frag_axis == 0) {
        // X
        //world_pos = frag_world_pos.zyx;
        //voxel_pos.z = voxel_resolution.z - voxel_pos.z;
        voxel_pos = voxel_pos.zyx;
    }
    else if (frag_axis == 1) {
        // Y
        //world_pos = frag_world_pos.zxy;
        //voxel_pos.z = voxel_resolution.z - voxel_pos.z;
        voxel_pos = voxel_pos.xzy;
    }
    else {
        // Z
        voxel_pos.z = voxel_resolution.z - voxel_pos.z;
    }

    imageStore(voxel_tex, voxel_pos, frag_albedo);
    //imageStore(voxel_tex, ivec3(world_pos), frag_albedo);
}