#version 450

in vec3 frag_world_pos;
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

    vec3 world_pos = frag_world_pos;
    vec3 voxel_pos = vec3(gl_FragCoord.xy, gl_FragCoord.z * voxel_resolution.z);
    if (frag_axis == 0) {
        world_pos = frag_world_pos.zyx;
        voxel_pos = voxel_pos.zyx;
    }
    else if (frag_axis == 1) {
        world_pos = frag_world_pos.zxy;
        voxel_pos = voxel_pos.zxy;
    }

    //voxel_pos.xy = 0.5*voxel_pos.xy+ 0.5;
    //imageStore(voxel_tex, ivec3(voxel_pos), frag_albedo);
    //imageStore(voxel_tex, ivec3(vec3(0.0, 0.0, 0.0) * imageSize(voxel_tex)), vec4(1.0));
    //imageStore(voxel_tex, voxel_pos, vec4(frag_albedo.rgb, 1.0));
    imageStore(voxel_tex, ivec3(voxel_pos.x, voxel_pos.y, voxel_pos.z), frag_albedo);
    //imageStore(voxel_tex, ivec3(0, 0, 0), vec4(1.0));
    //color = vec4(frag_albedo.rgb, 1.0);
    //color = vec4(voxel_pos / vec3(voxel_resolution), 1.0);
    //color = vec4(vec3(voxel_pos.z), 1.0);
}