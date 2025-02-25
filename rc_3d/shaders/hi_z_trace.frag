#version 450

in vec2 tex_coord;
out vec4 color;

uniform sampler2D hi_z_tex;

layout(std430) buffer HiZConstants {
    vec2 screen_res;
    vec2 screen_res_inv;
    vec2 hi_z_resolution;
    vec2 inv_hi_z_resolution;
    float hi_z_start_mip_level;
    float hi_z_max_mip_level;

    float max_steps;
    float far_z_depth;

    mat4 perspective;
    mat4 perspective_inv;
    mat4 viewport;
    mat4 viewport_inv;
};
const float DIR_EPS_X = 0.001;
const float DIR_EPS_Y = 0.001;
const float DIR_EPS_Z = 0.001;
const float HI_Z_STEP_EPS = 0.001;

float get_far_z_depth() {
    // TODO: Is this correct?
    return far_z_depth;
}

vec2 get_hi_z_pixel(vec2 point, vec2 level_size) {
    // TODO: Is this correct?
    return floor((point * screen_res_inv) * level_size);
}

// ray start and direction are in screen-space coordinates
// ray direction should not be normalised
void min_max_hi_z_traversal(
    vec2 step,
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

        const vec2 t_pixel_xy = ((pixel + step) / level_size + step_offset - ray_start.xy) * ray_dir_inv.xy;
        const float t_pixel_edge = min(t_pixel_xy.x, t_pixel_xy.y);

        vec2 scene_z_minmax = texelFetch(hi_z_tex, ivec2(pixel), int(mip_level)).rg;
        if (scene_z_minmax.y == 0.0) {
            // sampled out of bounds
            scene_z_minmax.xy = vec2(get_far_z_depth(), 0.0);
        }

        t_scene_z_minmax = (scene_z_minmax - ray_start.z) * ray_dir_inv.z;

        mip_level--;
        if (t_scene_z_minmax.x <= t_pixel_edge && t_param >= t_scene_z_minmax.y) {
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

    // Clamp step from the range [-1, 1] to [0, 1]
    vec2 step = clamp(step_sign.xy, 0.0, 1.0);

    vec3 ray_dir_inv = 1.0 / ray_dir;

    const vec2 starting_ray_pixel = get_hi_z_pixel(ray_start.xy, hi_z_resolution);
    const vec2 t_start_pixel_xy = ((starting_ray_pixel + step) / hi_z_resolution + step_offset - ray_start.xy) * ray_dir_inv.xy; 
    float t_param = min(t_start_pixel_xy.x, t_start_pixel_xy.y);
    vec2 t_scene_z_minmax = vec2(1.0, 0.0);
    float mip_level = hi_z_start_mip_level;

    min_max_hi_z_traversal(step, step_offset, ray_start, ray_dir, ray_dir_inv, mip_level, iters, t_param, t_scene_z_minmax);

    hit_point = vec3(ray_start + ray_dir * t_param);
    hit_point.xy *= hi_z_resolution;

    return (mip_level != -1.0) || ((t_param < t_scene_z_minmax.x || t_param > t_scene_z_minmax.y));
}

void main() {
    vec3 ray_start = vec3(0.0);
    vec3 ray_end = ray_start;
    vec3 hit_point;
    float iters = 0.0;
    bool hit = trace(ray_start, ray_end, iters, hit_point);
    color = vec4(vec3(hit), 1.0);
}