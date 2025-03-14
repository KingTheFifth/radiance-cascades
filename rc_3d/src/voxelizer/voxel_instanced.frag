#version 450

in vec3 voxel_pos;
in vec4 v_color;
uniform sampler3D voxel_tex;
out vec4 color;
uniform ivec3 voxel_resolution;

void main() {
    vec3 tex_coord = voxel_pos / vec3(voxel_resolution);
    //color = texture(voxel_tex, tex_coord);
    color = vec4(tex_coord, 1.0);
}