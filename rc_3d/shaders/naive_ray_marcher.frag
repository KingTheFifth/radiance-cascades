#version 450

in vec2 tex_coord;
uniform sampler2D scene_albedo;
uniform sampler2D scene_normal;
uniform sampler2D hi_z_tex;
out vec4 color;

const bool REMAP_DEPTH = false;

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

float screen_depth_to_view_depth(float depth) {
    // NOTE: These calculations depend on the projection matrix
    if (REMAP_DEPTH) {
        float remapped_depth = depth * 2.0 - 1.0;
        return - z_near * z_far / (z_far + remapped_depth * (z_near - z_far));
    }
    return - z_near * z_far / (z_far + depth * (z_near - z_far));
}

// pixel_coord.xy is the fragment coordinate (gl_FragCoord.xy),
// pixel_coord.z is the depth saved in the depth buffer for the pixel
vec4 screen_pos_to_view_pos(vec3 pixel_coord) {
    // Adapted from https://www.khronos.org/opengl/wiki/Compute_eye_space_from_window_space
    vec3 ndc = vec3(
        2.0 * pixel_coord.x * screen_res_inv.x - 1.0,
        2.0 * pixel_coord.y * screen_res_inv.y - 1.0,
        2.0 * pixel_coord.z - 1.0
    );

    float clip_w = perspective[3].z / (ndc.z - perspective[2].z / perspective[2].w);
    vec4 clip_pos = vec4(ndc.xyz * clip_w, clip_w);
    return perspective_inv * clip_pos;
}

vec4 view_pos_to_screen_pos(vec3 view_pos) {
    vec4 clip_pos = perspective * vec4(view_pos, 1.0);
    vec4 ndc = vec4(clip_pos.xyz / clip_pos.w, clip_pos.w);
    vec4 screen_pos = vec4(
        (ndc.x + 1.0) * 0.5 * screen_res.x,
        (ndc.y + 1.0) * 0.5 * screen_res.y,
        (ndc.z + 1.0) * 0.5,
        ndc.w
    );
    return screen_pos;
}

bool trace(vec3 ray_start_vs, vec3 direction_vs, out vec3 hit, out float iters) {
    hit = vec3(-1.0);
    ray_start_vs = ray_start_vs + direction_vs * 0.01;
    vec3 ray_end_vs = ray_start_vs + direction_vs * max_ray_distance;
    bool out_of_bounds = false;
    const float max_steps_inv = 1.0 / (max_steps - 1.0);

    for (float i = 0.0; i < max_steps && !out_of_bounds; i += 1.0) {
        iters++;
        float traveled_distance = i * max_steps_inv;
        vec3 ray_vs = mix(ray_start_vs, ray_end_vs, traveled_distance);
        vec3 ray_ss = view_pos_to_screen_pos(ray_vs).xyz;

        //float scene_z_min = textureLod(hi_z_tex, ray_ss.xy * screen_res_inv, 0).x;
        float scene_z_min = texelFetch(hi_z_tex, ivec2(ray_ss.xy), 0).x;
        out_of_bounds = (scene_z_min == 0.0);
        scene_z_min = screen_depth_to_view_depth(scene_z_min);
        float scene_z_max = scene_z_min + 3.0;

        //bool collides = (min_depth <= ray_vs.z && (max_depth+3.0) >= ray_vs.z);
        bool collides = (ray_vs.z >= scene_z_min && ray_vs.z <= scene_z_max);
        if (collides) {
            hit = ray_ss;
            return true;
        }
    }
    return false;
}

void main() {
    vec3 ray_start = vec3(gl_FragCoord.xy, texture(hi_z_tex, tex_coord).x);
    vec3 normal_vs = octahedral_decode(texture(scene_normal, tex_coord).xy);
    vec3 ray_start_vs = screen_pos_to_view_pos(ray_start).xyz;

    vec3 view_ray_vs = normalize(ray_start_vs);
    vec3 direction_vs = reflect(view_ray_vs, normal_vs);
    vec3 end_point_vs = ray_start_vs + direction_vs * max_ray_distance;
    vec3 ray_end = view_pos_to_screen_pos(end_point_vs).xyz;

    vec3 hit_point = vec3(-1.0, -1.0, 0.0);
    float iters = 0.0;
    bool hit = trace(ray_start_vs, direction_vs, hit_point, iters);
    vec4 hit_color = (!hit) ? vec4(0.0, 0.5, 0.5, 1.0) : texture(scene_albedo, hit_point.xy * screen_res_inv);

    color = hit_color;
}