#version 450

in vec2 tex_coord;

uniform sampler2D prev_cascade;
uniform sampler2D scene_albedo;
uniform sampler2D scene_emissive;
uniform sampler2D hi_z_tex;
uniform float cascade_index;

out vec4 color;

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

const float DIR_EPS_X = 0.001;
const float DIR_EPS_Y = 0.001;
const float DIR_EPS_Z = 0.001;
const float HI_Z_STEP_EPS = 0.001;
const bool REMAP_DEPTH = false;

const float altitudes[4] = {acos(-0.75), acos(-0.25), acos(0.25), acos(0.75)};
//const float altitudes[4] = {acos(0.75), acos(0.25), acos(-0.25), acos(-0.75)};

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

float get_far_z_depth() {
    // TODO: Is this correct?
    return z_far;
    //return 1.0;
}

vec2 get_hi_z_pixel(vec2 point, vec2 level_size) {
    // TODO: Is this correct?
    return floor((point * 1.0) * level_size);
}

// ray start and direction are in screen-space coordinates
// ray direction should not be normalised
void min_max_hi_z_traversal(
    vec2 step_length,
    vec2 step_offset,
    vec3 ray_start,
    vec3 ray_dir,
    vec3 ray_dir_inv,
    inout float mip_level,
    inout float iters,
    inout float t_param,
    inout vec2 t_scene_z_minmax
) {
    while (mip_level >= 0.0 && iters < max_steps && t_param <= 1.0) {
        iters++;
        const vec2 max_ray_point_xy = ray_start.xy + ray_dir.xy * t_param;

        const vec2 level_size = vec2(textureSize(hi_z_tex, int(mip_level)).xy);
        const vec2 pixel = get_hi_z_pixel(max_ray_point_xy, level_size);

        const vec2 t_pixel_xy = ((pixel + step_length) / level_size + step_offset - ray_start.xy) * ray_dir_inv.xy;
        const float t_pixel_edge = min(t_pixel_xy.x, t_pixel_xy.y);

        vec2 scene_z_minmax = texelFetch(hi_z_tex, ivec2(pixel), int(mip_level)).rg;
        if (scene_z_minmax.y == 0.0) {
            // sampled out of bounds
            scene_z_minmax.xy = vec2(get_far_z_depth(), 0.0);
        }

        t_scene_z_minmax = (scene_z_minmax.xy - ray_start.z) * ray_dir_inv.z;

        mip_level--;
        if (t_scene_z_minmax.x <= t_pixel_edge && t_param <= t_scene_z_minmax.y) {
            // Hit at current mip level, go down to next one
            t_param = max(t_param, t_scene_z_minmax.x);
        }
        else {
            // Miss, go up to higher mip level
            t_param = t_pixel_edge;
            mip_level = min(hi_z_max_mip_level, mip_level + 2.0);
        }
    }
}

// ray_start, ray_end and hit point are all in screen space
// returns true if hit, false if miss
bool trace(vec3 ray_start, vec3 ray_end, inout float iters, out vec3 hit_point) {
    
    // Map ray ray_end point from (pixel coordinate, depth) to (UV coordinate, depth)
    ray_start.xy *= inv_hi_z_resolution;
    ray_end.xy *= inv_hi_z_resolution;

    vec3 ray_dir = ray_end - ray_start;
    vec3 step_sign;
    step_sign.x = (ray_dir.x >= 0) ? 1.0 : -1.0;
    step_sign.y = (ray_dir.y >= 0) ? 1.0 : -1.0;
    step_sign.z = (ray_dir.z >= 0) ? 1.0 : -1.0;
    vec2 step_offset = step_sign.xy * (HI_Z_STEP_EPS * inv_hi_z_resolution);

    // Ignore zero components => no divide by 0
    vec3 ray_dir_abs = abs(ray_dir);
    ray_dir.x = (ray_dir_abs.x < DIR_EPS_X) ? DIR_EPS_X * step_sign.x : ray_dir.x;
    ray_dir.y = (ray_dir_abs.y < DIR_EPS_Y) ? DIR_EPS_Y * step_sign.y : ray_dir.y;
    ray_dir.z = (ray_dir_abs.z < DIR_EPS_Z) ? DIR_EPS_Z * step_sign.z : ray_dir.z;

    // Clamp step_length from the range [-1, 1] to [0, 1]
    vec2 step_length = clamp(step_sign.xy, 0.0, 1.0);

    vec3 ray_dir_inv = 1.0 / ray_dir;

    const vec2 starting_ray_pixel = get_hi_z_pixel(ray_start.xy, hi_z_resolution);
    const vec2 t_start_pixel_xy = ((starting_ray_pixel + step_length) / hi_z_resolution + step_offset - ray_start.xy) * ray_dir_inv.xy; 
    float t_param = min(t_start_pixel_xy.x, t_start_pixel_xy.y);
    vec2 t_scene_z_minmax = vec2(1.0, 0.0);
    float mip_level = hi_z_start_mip_level;

    min_max_hi_z_traversal(step_length, step_offset, ray_start, ray_dir, ray_dir_inv, mip_level, iters, t_param, t_scene_z_minmax);

    hit_point = vec3(ray_start + ray_dir * t_param);
    hit_point.xy *= hi_z_resolution;

    return ((mip_level != -1.0) || ((t_param < t_scene_z_minmax.x || t_param > t_scene_z_minmax.y)));
}

vec3 linear_to_srgb(vec3 c) {
    return pow(c.rgb, vec3(1.6));
}

vec3 srgb_to_linear(vec3 c) {
    return pow(c.rgb, vec3(1.0 / 1.6));
}

vec4 trace_radiance(vec3 ray_start_ws, vec3 ray_dir_ws, float interval_length) {
    const vec3 ray_end_ws = ray_start_ws + ray_dir_ws * interval_length;

    const vec3 ray_start_ss = view_pos_to_screen_pos((world_to_view * vec4(ray_start_ws, 1.0)).xyz).xyz;
    const vec3 ray_end_ss = view_pos_to_screen_pos((world_to_view * vec4(ray_end_ws, 1.0)).xyz).xyz;

    vec3 hit_point_ss = vec3(-1.0, -1.0, 0.0);
    float iters = 0.0;
    bool missed = trace(ray_start_ss, ray_end_ss, iters, hit_point_ss);
    // Alpha channel tracks occlusion such that 0.0 means the ray hit an occluder
    return missed ? vec4(vec3(0.0), 1.0) : vec4(linear_to_srgb(texture(scene_albedo, hit_point_ss.xy * screen_res_inv).rgb), 0.0);
}

vec4 merge(vec4 radiance, vec2 dir_index, vec2 dir_block_size, vec2 coord_within_block) {
    if (radiance.a == 0.0 || cascade_index >= num_cascades - 1.0) {
        return vec4(radiance.rgb, 1.0 - radiance.a);
    }

    const vec2 prev_num_dirs = vec2(
        pow(2.0, cascade_index + 1.0),
        4.0
    );

    const vec2 prev_dir_block_size = floor(screen_res / (c0_probe_spacing * pow(2.0, cascade_index + 1.0)));
    const vec2 prev_cascade_res = prev_num_dirs * prev_dir_block_size;
    // Calculate bottom-left pixel of the direction block of the higher cascade to merge with
    vec2 interpolation_point = dir_index * prev_dir_block_size;
    // Add an offset to interpolate from the 4 closest probes in the higher cascade
    interpolation_point += clamp(0.5 * coord_within_block + 0.25, vec2(0.5), prev_dir_block_size - 0.5);
    return radiance += texture(prev_cascade, interpolation_point * (1.0 / prev_cascade_res));
}

void main() {
    const float num_altitudinal_rays = 4.0;
    const float num_azimuthal_rays = 2.0 * pow(2.0, cascade_index);

    const vec2 probe_spacing = vec2(c0_probe_spacing * pow(2.0, cascade_index));
    const vec2 probe_count = floor(screen_res / probe_spacing); // This is also the size of a direction block
    const vec2 cascade_res = vec2(
        probe_count.x * num_azimuthal_rays,
        probe_count.y * num_altitudinal_rays
    );  // Note: the width should remain constant and the height should half for every cascade

    const vec2 pixel_coord = floor(tex_coord * cascade_res);
    const vec2 coord_within_dir_block = mod(pixel_coord, probe_count);
    const vec2 dir_block_index = floor(pixel_coord / probe_count);

    const float interval_length = c0_interval_length * pow(4.0, cascade_index);
    //const float interval_length = 40.0;
    const float interval_start = c0_interval_length * ((1.0 - pow(4.0, cascade_index)) / (1.0 - 4.0));

    const vec2 probe_pixel = (coord_within_dir_block + 0.5) * probe_spacing; // Probes in center of pixel
    const vec3 min_probe_pos_ss = vec3(probe_pixel, textureLod(hi_z_tex, probe_pixel * screen_res_inv, 0).r);
    const vec3 min_probe_pos_ws = (world_to_view_inv * vec4(screen_pos_to_view_pos(min_probe_pos_ss).xyz, 1.0)).xyz;
    //const vec3 probe_pos_max = vec3(probe_pixel_pos, textureLod(hi_z_tex, probe_pixel_pos, 0).g);

    color = vec4(0.0);

    if (min_probe_pos_ss.z >= 0.99999) {
        // Do not calculate probes placed in the sky/out of bounds
        return;
    }

    for (float i = 0.0; i < 2.0; i++) {
        const float preavg_azimuth_index = dir_block_index.x * 2.0 + i;
        const float ray_azimuth = (preavg_azimuth_index + 0.5) * (2.0 * 3.14169266 / (num_azimuthal_rays * 2.0));
        const float ray_altitude = altitudes[int(dir_block_index.y)];

        const vec3 ray_dir_ws = vec3(
            cos(ray_azimuth)*sin(ray_altitude),
            cos(ray_altitude),
            sin(ray_azimuth)*sin(ray_altitude)
        ); //TODO: invert sign of some component?

        // TODO: Trace both min and max depth probes at the same time somehow
        const vec3 ray_start_ws = min_probe_pos_ws + ray_dir_ws * interval_start;
        //const vec3 ray_max_start = probe_pos_max + ray_dir * interval_start;

        vec4 radiance_min = trace_radiance(ray_start_ws, ray_dir_ws, interval_length);
        color += merge(radiance_min, dir_block_index, probe_count, coord_within_dir_block) * 0.25;
    }

    //color = vec4(screen_pos_to_view_pos(probe_pos_min).xyz, 1.0);
    //color = vec4(dir_block_index / vec2(num_azimuthal_rays, num_altitudinal_rays), 0.0, 1.0);
    //color = vec4(coord_within_dir_block / probe_count, 0.0, 1.0);
}