#version 450

in vec4 frag_world_pos;
in vec3 frag_normal;
in vec4 frag_albedo;
flat in int frag_axis;

layout(binding = 0, rgba16f) uniform writeonly image3D voxel_tex;

uniform ivec3 voxel_resolution;

out vec4 color;

void main() {

    ivec3 voxel_pos = ivec3(gl_FragCoord.xy, (gl_FragCoord.z * 2.0 - 1.0) * voxel_resolution.z);

    // Rotate the voxel position to one viewed from a projection along the Z-axis
    // This is needed since the fragment may have been projected along another axis in the
    // geometry shader to maximise triangle surface area
    if (frag_axis == 0) {
        // X
        voxel_pos = voxel_pos.zyx;
    }
    else if (frag_axis == 1) {
        // Y
        voxel_pos = voxel_pos.xzy;
    }
    else {
        // Z
        voxel_pos.z = voxel_resolution.z - voxel_pos.z;
    }

    // TODO: Store normals in a separate 3D texture
    // TODO: Store through atomic averaging as several fragments may belong to the same voxel
    imageStore(voxel_tex, voxel_pos, frag_albedo);
}