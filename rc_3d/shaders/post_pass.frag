#version 450

in vec2 tex_coord;
out vec4 color;

uniform sampler2D merged_cascade_0;
uniform sampler2D scene_normal;

layout(std430) readonly buffer Constants {
    vec2 screen_res;
    vec2 screen_res_inv;

    // Hi Z screen-space ray marching
    vec2 hi_z_resolution;
    vec2 inv_hi_z_resolution;
    mat4 world_to_view;
    mat4 world_to_view_inv;
    mat4 perspective;
    mat4 perspective_inv;
    float hi_z_start_mip_level;
    float hi_z_max_mip_level;
    float max_steps;
    float max_ray_distance;
    float z_far;
    float z_near;

    // Radiance cascades
    float num_cascades;
    float c0_probe_spacing;
    float c0_interval_length;
    vec2 c0_resolution;
};

const float altitudes[4] = {acos(-0.75), acos(-0.25), acos(0.25), acos(0.75)};

vec3 srgb_to_linear(vec3 c) {
    return pow(c.rgb, vec3(1.0 / 1.6));
}

void main() {
    color = vec4(srgb_to_linear(texture(merged_cascade_0, tex_coord).rgb), 1.0);
}