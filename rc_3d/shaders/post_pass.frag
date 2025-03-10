#version 450

in vec2 tex_coord;
out vec4 color;

uniform sampler2D merged_cascade_0;
uniform sampler2D scene_normal;
uniform sampler2D scene_albedo;

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

vec3 octahedral_decode(vec2 v) {
    // Based on https://knarkowicz.wordpress.com/2014/04/16/octahedron-normal-vector-encoding/
    //vec2 v_adjusted = 2.0 * v - 1.0;
    vec2 v_adjusted = v;
    vec3 n = vec3(v_adjusted.xy, 1.0 - abs(v_adjusted.x) - abs(v_adjusted.y));
    float t = max((-n.z), 0.0);
    return normalize(vec3(
        n.x + ((n.x >= 0.0) ? (-t) : t),
        n.y + ((n.y >= 0.0) ? (-t) : t),
        n.z
    ));
}

void main() {
    vec3 radiance = vec3(0.0);
    vec3 normal = octahedral_decode(texture(scene_normal, tex_coord).xy);
    for (float alt = 0.0; alt < 4.0; alt += 1.0) {
        vec2 r1_coord = vec2(tex_coord.x * 0.5, tex_coord.y * 0.25 + alt * 0.25);
        vec2 r2_coord = vec2(tex_coord.x * 0.5 + 0.5, tex_coord.y * 0.25 + alt * 0.25);
        vec3 r1 = texture(merged_cascade_0, r1_coord).rgb;
        vec3 r2 = texture(merged_cascade_0, r2_coord).rgb;

        float altitude = altitudes[int(alt)]; 
        vec3 r1_dir = normalize(mat3(world_to_view) * vec3(
            cos(3.14159265 * 0.5) * sin(altitude),
            cos(altitude),
            sin(3.14159265 * 0.5) * sin(altitude)
        ));
        vec3 r2_dir = normalize(mat3(world_to_view) * vec3(
            cos(3.14159265 * 1.5) * sin(altitude),
            cos(altitude),
            sin(3.14159265 * 1.5) * sin(altitude)
        ));

        radiance += (r1 * dot(r1_dir, normal) + r2 * dot(r2_dir, normal));
    }
    vec4 albedo = texture(scene_albedo, tex_coord);
    color = vec4(srgb_to_linear(albedo.rgb * radiance), albedo.a);
    //color = vec4(srgb_to_linear(texture(merged_cascade_0, tex_coord).rgb), 1.0);
}