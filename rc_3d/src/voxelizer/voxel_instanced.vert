#version 450

in vec3 position;
out vec3 voxel_pos;

uniform mat4 projection;
uniform mat4 world_to_view;
uniform ivec3 voxel_resolution;

void main() {
    voxel_pos = vec3(
        -gl_InstanceID % voxel_resolution.x,
        gl_InstanceID / (voxel_resolution.x * voxel_resolution.z),
        (gl_InstanceID / voxel_resolution.x) % voxel_resolution.z 
        );
    //gl_Position = projection * world_to_view * vec4((position + voxel_pos) * 0.05, 1.0); 
    gl_Position = projection * world_to_view * vec4((position + voxel_pos - voxel_resolution * 0.5) * 0.05, 1.0); 
}