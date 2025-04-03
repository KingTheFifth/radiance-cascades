#version 450

in vec4 frag_world_pos;
in vec3 frag_normal;
in vec4 frag_albedo;
in vec4 frag_emissive;
in vec2 frag_tex_coord;
flat in int frag_axis;

layout(binding = 0, rgba16f) uniform writeonly image3D voxel_tex;

uniform ivec3 voxel_resolution;

uniform vec3 emissive;
uniform int has_emissive;
uniform vec3 diffuse;
uniform int has_diffuse;
uniform vec3 specular;
uniform int has_specular;
uniform float opacity;
uniform int has_opacity;

uniform sampler2D diffuse_tex;
uniform int has_diffuse_tex;
uniform sampler2D specular_tex;
uniform int has_specular_tex;
uniform sampler2D normal_tex;
uniform int has_normal_tex;
uniform sampler2D opacity_tex;
uniform int has_opacity_tex;

out vec4 color;

void main() {
    vec2 adjusted_tex_coord = vec2(frag_tex_coord.x, 1.0 - frag_tex_coord.y);

    vec4 diffuse_color = vec4(0.0);
    if (has_opacity_tex == 1 && texture(opacity_tex, adjusted_tex_coord).r < 0.1) {
        discard;
    }
    if (has_diffuse_tex == 1) {
        diffuse_color = texture(diffuse_tex, adjusted_tex_coord);
    }
    else if (has_diffuse == 1) {
        diffuse_color = vec4(diffuse, 1.0);
    }
    else {
        diffuse_color = frag_albedo;
    }

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
    //imageStore(voxel_tex, voxel_pos, diffuse_color);
    imageStore(voxel_tex, voxel_pos, vec4(emissive, 1.0));
}