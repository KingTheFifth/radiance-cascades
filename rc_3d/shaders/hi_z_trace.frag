#version 450

in vec2 tex_coord;
out vec4 color;

uniform sampler2D hi_z_tex;
uniform sampler2D scene_albedo;
uniform sampler2D scene_normal;
uniform sampler2D scene_vs_position;

layout(std430) buffer HiZConstants {
    vec2 screen_res;
    vec2 screen_res_inv;
    vec2 hi_z_resolution;
    vec2 inv_hi_z_resolution;
    float hi_z_start_mip_level;
    float hi_z_max_mip_level;

    float max_steps;
    float z_far;

    mat4 perspective;
    mat4 perspective_inv;
    float z_near;
    float max_ray_distance;
};
const float DIR_EPS_X = 0.001;
const float DIR_EPS_Y = 0.001;
const float DIR_EPS_Z = 0.001;
const float HI_Z_STEP_EPS = 0.001;
const bool REMAP_DEPTH = false;

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

vec4 ssr() {
    vec3 ray_start = vec3(gl_FragCoord.xy, texture(hi_z_tex, tex_coord).r);
    vec3 normal_vs = texture(scene_normal, tex_coord).xyz;

    //vec3 origin_vs = texture(scene_vs_position, tex_coord).xyz;
    vec3 ray_start_vs = screen_pos_to_view_pos(ray_start).xyz;

    vec3 view_ray_vs = normalize(ray_start_vs);
    vec3 direction_vs = reflect(view_ray_vs, normal_vs);
    vec3 end_point_vs = ray_start_vs + direction_vs * max_ray_distance;
    vec3 ray_end = view_pos_to_screen_pos(end_point_vs).xyz;

    vec3 hit_point = vec3(-1.0, -1.0, 0.0);
    float iters = 0.0;
    bool missed = true;
    // TODO: Figure out why this condition does not work
    if (true || direction_vs.z > 0.0) {
        missed = trace(ray_start, ray_end, iters, hit_point);
    }

    vec4 hit_color = missed ? vec4(0.0, 0.5, 0.5, 1.0) : texture(scene_albedo, hit_point.xy * screen_res_inv);
    return hit_color;
}

void main() {
    vec4 albedo = texture(scene_albedo, tex_coord);
    color = (all(greaterThanEqual(albedo, vec4(1.0 - 0.001)))) ? ssr() : albedo;
}