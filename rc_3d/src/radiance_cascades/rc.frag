#version 450

in vec2 tex_coord;

uniform sampler2D prev_cascade;
uniform sampler2D scene_albedo;
uniform sampler2D scene_emissive;
uniform sampler2D scene_normal; // View space
uniform sampler2D hi_z_tex;
uniform float cascade_index;

uniform bool merge_cascades;

out vec4 color;

#define NAIVE_SS 0
#define HI_Z 1
#define VOXEL 2
#define TRACE_METHOD VOXEL

#define MISS_COLOR vec4(0.0, 0.0, 0.0, 1.0)

// Uncomment to use c0 interval length for all cascades
//#define DEBUG_INTERVALS

layout(std430) readonly buffer RCConstants {
    vec2 c0_resolution;
    float num_cascades;
    float c0_probe_spacing;
    float c0_interval_length;
    float normal_offset;    // Offset probe position along surface normals
    float gamma;
    float ambient_occlusion_factor;
    float diffuse_intensity;
    float ambient_occlusion;
};

layout(std430) readonly buffer HiZConstants {
    vec2 hi_z_resolution;
    vec2 inv_hi_z_resolution;
    float hi_z_start_mip_level;
    float hi_z_max_mip_level;
    float max_steps;
    float max_ray_distance;
    float z_far;
    float z_near;
};

layout(std430) readonly buffer SceneMatrices {
    mat4 world_to_view;
    mat4 world_to_view_inv;
    mat4 perspective;
    mat4 perspective_inv;
    vec2 screen_res;
    vec2 screen_res_inv;
};

uniform float step_count;
uniform mat4 world_to_voxel;
uniform vec3 voxel_resolution;
layout(binding = 0, rgba16f) uniform readonly image3D voxel_tex;

const float DIR_EPS_X = 0.001;
const float DIR_EPS_Y = 0.001;
const float DIR_EPS_Z = 0.001;
const float HI_Z_STEP_EPS = 0.01;
const bool REMAP_DEPTH = true;

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

float linearize_depth(float depth) {
    return -1.0 * screen_depth_to_view_depth(depth) / (z_far - z_near);
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
bool trace_hi_z(vec3 ray_start, vec3 ray_end, inout float iters, out vec3 hit_point) {
    // Based on "Screen Space Reflection Techniques" by Anthony Paul Beug
    // https://ourspace.uregina.ca/server/api/core/bitstreams/14fd32d7-de0c-4da0-9fda-98892b57469c/content
    
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
    return pow(c.rgb, vec3(1.0 / gamma));
}

vec3 srgb_to_linear(vec3 c) {
    return pow(c.rgb, vec3(gamma));
}

vec4 trace_radiance_hi_z(vec3 ray_start_vs, vec3 ray_dir_vs, float interval_length) {
    const vec3 ray_end_vs = ray_start_vs + ray_dir_vs * interval_length;

    const vec3 ray_start_ss = view_pos_to_screen_pos(ray_start_vs).xyz;
    const vec3 ray_end_ss = view_pos_to_screen_pos(ray_end_vs).xyz;

    vec3 hit_point_ss = vec3(-1.0, -1.0, 0.0);
    float iters = 0.0;
    bool missed = trace_hi_z(ray_start_ss, ray_end_ss, iters, hit_point_ss);
    // Alpha channel tracks occlusion such that 0.0 means the ray hit an occluder
    return missed ? MISS_COLOR : vec4(linear_to_srgb(texture(scene_emissive, hit_point_ss.xy * screen_res_inv).rgb), 0.0);
}

vec4 trace_radiance_naive_screen_space(vec3 ray_start_vs, vec3 ray_dir_vs, float interval_length) {
    vec3 ray_end_vs = ray_start_vs + ray_dir_vs * interval_length;

    float steps = float(20 * (1 << (int(cascade_index) + 1)));  // TODO: Make configurable
    float step_count_inv = 1.0 / (steps - 1.0);
    for (float i = 0.0; i < steps; i++) {
        float traveled_distance = i * step_count_inv;
        vec3 ray_vs = mix(ray_start_vs, ray_end_vs, traveled_distance);
        vec3 ray_ss = view_pos_to_screen_pos(ray_vs).xyz;

        if (any(lessThan(ray_ss.xy, vec2(0.0))) || any(greaterThan(ray_ss.xy, screen_res))) {
            break;
        }

        vec2 min_max_depth = texelFetch(hi_z_tex, ivec2(ray_ss.xy), 0).xy;
        bool collides = (ray_ss.z >= min_max_depth.x && ray_ss.z <= min_max_depth.y);
        if (collides) {
            return vec4(srgb_to_linear(texture(scene_emissive, ray_ss.xy * screen_res_inv).rgb), 0.0);
        }
    }
    return MISS_COLOR;
}

vec4 trace_radiance_voxel(vec3 ray_start_ws, vec3 ray_dir_ws, float interval_length) {
    const vec3 ray_end_ws = ray_start_ws + ray_dir_ws * interval_length;
    const float step_count_inv = 1.0 / (step_count - 1.0);
    for (float s = 0.0; s < step_count; s++) {
        const vec3 curr_point = mix(ray_start_ws, ray_end_ws, s * step_count_inv);
        const vec3 voxel = (world_to_voxel * vec4(curr_point, 1.0)).xyz;
        const vec4 curr_sample = imageLoad(voxel_tex, ivec3(voxel));
        if (any(lessThan(voxel, vec3(0.0))) || any(greaterThanEqual(voxel, voxel_resolution))) {
            return MISS_COLOR;
        }
        if (curr_sample.a > 0.05) {
            return vec4(srgb_to_linear(curr_sample.rgb), 0.0);
        }
    }
    return MISS_COLOR;
}

vec4 get_upper_depth_weights(vec3 probe_pos_ss, vec2 coord_within_block) {
    const vec2 upper_probe_spacing = vec2(c0_probe_spacing * pow(2.0, cascade_index + 1.0));
    const vec2 upper_probe_count = floor(screen_res / upper_probe_spacing);

    vec2 upper_coord_within_block = 0.5 * coord_within_block;
    ivec2 upper_probe_base = ivec2(floor(upper_coord_within_block));
    vec2 upper_probe_frac = fract(upper_coord_within_block);
    vec4 bilinear_weights = vec4(
        (1.0 - upper_probe_frac.x) * (1.0 - upper_probe_frac.y),
        upper_probe_frac.x * (1.0 - upper_probe_frac.y),
        (1.0 - upper_probe_frac.x) * upper_probe_frac.y,
        upper_probe_frac.x * upper_probe_frac.y
    );

    ivec2 upper_probe_offsets[4] = {ivec2(0.0, 0.0), ivec2(1.0, 0.0), ivec2(0.0, 1.0), ivec2(1.0, 1.0)};
    ivec2 upper_probe_coords[4] = {
        upper_probe_base + upper_probe_offsets[0],
        upper_probe_base + upper_probe_offsets[1],
        upper_probe_base + upper_probe_offsets[2],
        upper_probe_base + upper_probe_offsets[3]
    };

    ivec2 upper_probe_screen_coords[4] = {
        upper_probe_coords[0] * int(upper_probe_spacing),
        upper_probe_coords[1] * int(upper_probe_spacing),
        upper_probe_coords[2] * int(upper_probe_spacing),
        upper_probe_coords[3] * int(upper_probe_spacing)
    };

    float linear_probe_depth = linearize_depth(probe_pos_ss.z);
    vec4 depths = vec4(
        linearize_depth(texelFetch(hi_z_tex, upper_probe_screen_coords[0], 0).r),
        linearize_depth(texelFetch(hi_z_tex, upper_probe_screen_coords[1], 0).r),
        linearize_depth(texelFetch(hi_z_tex, upper_probe_screen_coords[2], 0).r),
        linearize_depth(texelFetch(hi_z_tex, upper_probe_screen_coords[3], 0).r)
    );
    float min_depth = min(min(depths.x, depths.y), min(depths.z, depths.w));
    float max_depth = max(max(depths.x, depths.y), max(depths.z, depths.w));
    float depth_diff = max_depth - min_depth;
    float avg_depth = dot(depths, vec4(0.25));
    bool depth_edge = (depth_diff / avg_depth) > 0.1;

    vec4 weights = bilinear_weights;
    if (depth_edge) {
        vec4 dd = abs(depths - vec4(linear_probe_depth));
        weights *= vec4(1.0) / (dd + vec4(0.0001));
    }
    weights /= dot(weights, vec4(1.0));

    return weights;
}

vec4 merge(vec4 radiance, vec2 dir_index, vec3 probe_pos_ss, vec2 coord_within_block) {
    if (radiance.a == 0.0 || cascade_index >= num_cascades - 1.0) {
        return vec4(radiance.rgb, 1.0 - radiance.a);
    }

    // (Number of azimuthal directions increase by 2 each higher cascade, altitudinal stay the same)
    const vec2 upper_cascade_num_dirs = vec2(
        4.0 * pow(2.0, cascade_index + 1.0),
        4.0 * pow(2.0, cascade_index + 1.0)
    );

    // (Number of probes decrease by 2 each higher cascade)
    const vec2 upper_probe_spacing = vec2(c0_probe_spacing * pow(2.0, cascade_index + 1.0));
    const vec2 upper_cascade_probe_count = floor(screen_res / upper_probe_spacing);
    const vec2 upper_cascade_res = upper_cascade_num_dirs * upper_cascade_probe_count;
    const vec2 upper_cascade_res_inv = 1.0 / upper_cascade_res;

    vec2 upper_coord_within_block = 0.5 * coord_within_block;
    ivec2 upper_probe_base = ivec2(floor(upper_coord_within_block));

    ivec2 upper_probe_offsets[4] = {ivec2(0.0, 0.0), ivec2(1.0, 0.0), ivec2(0.0, 1.0), ivec2(1.0, 1.0)};
    ivec2 upper_probe_coords[4] = {
        upper_probe_base + upper_probe_offsets[0],
        upper_probe_base + upper_probe_offsets[1],
        upper_probe_base + upper_probe_offsets[2],
        upper_probe_base + upper_probe_offsets[3]
    };

    ivec2 upper_probe_screen_coords[4] = {
        upper_probe_coords[0] * int(upper_probe_spacing),
        upper_probe_coords[1] * int(upper_probe_spacing),
        upper_probe_coords[2] * int(upper_probe_spacing),
        upper_probe_coords[3] * int(upper_probe_spacing)
    };

    vec4 weights = get_upper_depth_weights(probe_pos_ss, coord_within_block);

    // Merge this ray direction with the two closest directions in the upper cascade
    vec4 upper_radiance = vec4(0.0);
    for (float alt = 0.0; alt < 2.0; alt++) {
        for (float azi = 0.0; azi < 2.0; azi++) {
            vec2 branched_dir_index = dir_index * vec2(2.0, 2.0) + vec2(azi, alt);
            vec2 interpolation_point = branched_dir_index * upper_cascade_probe_count; // Bottom left probe texel

            vec4 dir_radiance = vec4(0.0);
            for (int probe = 0; probe < 4; probe++) {
                vec2 p = clamp(upper_probe_coords[probe], vec2(0.5), upper_cascade_probe_count - 0.5);
                //vec4 s = texture(prev_cascade, (interpolation_point + p) * upper_cascade_res_inv);  //TODO: Should this be a texel fetch?
                vec4 s = texelFetch(prev_cascade, ivec2(interpolation_point + p), 0);

                dir_radiance += s * weights[probe];
            }

            upper_radiance += dir_radiance;
        }
    }
    return radiance + upper_radiance * 0.25;
}

void main() {
    // Note: These factors for increasing the number of directions and the probe spacing
    // ensure that all cascades have the same dimensions which is nice to work with
    const float num_altitudinal_rays = 4.0 * pow(2.0, cascade_index);
    const float num_azimuthal_rays = 4.0 * pow(2.0, cascade_index);
    const vec2 probe_spacing = vec2(c0_probe_spacing * pow(2.0, cascade_index));

    const vec2 probe_count = floor(screen_res / probe_spacing); // This is also the size of a direction block
    const vec2 cascade_res = vec2(
        probe_count.x * num_azimuthal_rays,
        probe_count.y * num_altitudinal_rays
    );

    const vec2 pixel_coord = gl_FragCoord.xy;
    const vec2 coord_within_dir_block = mod(pixel_coord, probe_count);
    const vec2 dir_block_index = floor(pixel_coord / probe_count);

    const vec2 probe_pixel = (coord_within_dir_block + 0.5) * probe_spacing; // Probes in center of pixel
    const vec3 normal_ws = octahedral_decode(texture(scene_normal, probe_pixel * screen_res_inv).xy);
    const vec3 normal_vs = normalize(mat3(world_to_view) * normal_ws);
    const vec3 min_probe_pos_ss = vec3(probe_pixel, textureLod(hi_z_tex, probe_pixel * screen_res_inv, 0).r);
    //const vec3 min_probe_pos_vs = screen_pos_to_view_pos(min_probe_pos_ss).xyz + normal_vs * normal_offset;
    const vec3 min_probe_pos_vs = screen_pos_to_view_pos(min_probe_pos_ss).xyz;
    const vec3 min_probe_pos_ws = (world_to_view_inv * vec4(min_probe_pos_vs, 1.0)).xyz;
    //const vec3 probe_pos_max = vec3(probe_pixel_pos, textureLod(hi_z_tex, probe_pixel_pos, 0).g);

    #ifdef DEBUG_INTERVALS
    const float interval_length = c0_interval_length;
    const float interval_start = c0_interval_length * cascade_index;
    #else
    const float interval_length = c0_interval_length * pow(2.0, cascade_index);
    const float interval_start = c0_interval_length * ((1.0 - pow(2.0, cascade_index)) / (1.0 - 2.0));
    #endif

    if (min_probe_pos_ss.z >= 0.99999) {
        // Do not calculate probes placed in the sky/out of bounds
        color = MISS_COLOR;
        return;
    }

    const float ray_azimuth = (dir_block_index.x + 0.5) * (2.0 * 3.14169265 / (num_azimuthal_rays));
    const float ray_altitude = (dir_block_index.y + 0.5) * (3.14169265 / (num_altitudinal_rays));
    //const float ray_altitude = altitudes[int(dir_block_index.y)];
    const vec3 ray_dir_ws = normalize(vec3(
        cos(ray_azimuth)*sin(ray_altitude),
        cos(ray_altitude),
        sin(ray_azimuth)*sin(ray_altitude)
    ));
    const vec3 ray_dir_vs = normalize(mat3(world_to_view) * ray_dir_ws);

    // TODO: Trace both min and max depth probes at the same time somehow
    const vec3 ray_start_ws = min_probe_pos_ws + ray_dir_ws * interval_start + normal_ws * normal_offset;
    const vec3 ray_start_vs = min_probe_pos_vs + ray_dir_vs * interval_start + normal_vs * normal_offset;
    // const vec3 ray_start_ws = min_probe_pos_ws + ray_dir_ws * interval_start;
    // const vec3 ray_start_vs = min_probe_pos_vs + ray_dir_vs * interval_start;

    #if (TRACE_METHOD == NAIVE_SS)
    vec4 radiance_min = trace_radiance_naive_screen_space(ray_start_vs, ray_dir_vs, interval_length);
    #elif (TRACE_METHOD == HI_Z)
    vec4 radiance_min = trace_radiance_hi_z(ray_start_vs, ray_dir_vs, interval_length);
    #elif (TRACE_METHOD == VOXEL)
    vec4 radiance_min = trace_radiance_voxel(ray_start_ws, ray_dir_ws, interval_length);
    #else
    #error "Invalid tracing method"
    #endif

    vec4 unmerged_radiance = radiance_min;
    vec4 merged_radiance = merge(radiance_min, dir_block_index, min_probe_pos_ss, coord_within_dir_block);
    color = merge_cascades ? merged_radiance : unmerged_radiance;

    //color = vec4(dir_block_index / vec2(num_azimuthal_rays, num_altitudinal_rays), 0.0, 1.0);
    //color = vec4(coord_within_dir_block / probe_count, 0.0, 1.0);
    //color = vec4(ray_dir_vs, 1.0);
    //color = vec4(normal_vs, 1.0);
    //color = vec4(min_probe_pos_ws / 5.0, 1.0);
}