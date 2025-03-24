#version 450

in vec4 frag_world_pos;
in vec3 frag_normal;
in vec4 frag_albedo;
in vec4 frag_emissive;
flat in int frag_axis;

layout(binding = 0, rgba16f) uniform writeonly image3D voxel_tex;
layout(binding = 1, rg16f) uniform writeonly image3D voxel_normal;

uniform ivec3 voxel_resolution;

out vec4 color;

vec2 sign_not_zero(vec2 v) {
    return vec2(
        (v.x >= 0.0) ? 1.0 : -1.0,
        (v.y >= 0.0) ? 1.0 : -1.0
    );
}

vec2 octahedral_encode(vec3 v) {
    // Based on https://knarkowicz.wordpress.com/2014/04/16/octahedron-normal-vector-encoding/
    vec2 n = v.xy;
    n = n * (1.0 / (abs(v.x) + abs(v.y) + abs(v.z))); 
    n = (v.z < 0.0) ? ((vec2(1.0) - abs(n.yx)) * sign_not_zero(n)) : n.xy;
    //return n * 0.5 + 0.5;
    return n;
}

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

    vec2 encoded_normal = octahedral_encode(normalize(frag_normal));

    // TODO: Store normals in a separate 3D texture
    // TODO: Store through atomic averaging as several fragments may belong to the same voxel
    //imageStore(voxel_tex, voxel_pos, frag_albedo);
    imageStore(voxel_tex, voxel_pos, frag_emissive);
    imageStore(voxel_normal, voxel_pos, vec4(encoded_normal, 0.0, 1.0));
}