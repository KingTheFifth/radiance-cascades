#version 450

in vec2 tex_coord;
out vec4 color;

const float EPSILON = 0.0001;

// RC params ---------------------------------------------
uniform sampler2D scene;
uniform sampler2D dist_field;
uniform sampler2D prev_cascade;
uniform vec2 screen_dimensions;
uniform vec2 cascade_dimensions;
uniform float num_cascades;
uniform float cascade_index;          // Current cascade

// These get scaled for each cascade: 0.25x density and 4x rays for each cascade in comparison to previous one
// (Other factors are possible but this keeps the cascade dimensions equal for all cacades)
uniform float c0_probe_density;     // As power of 2
uniform float c0_interval_length;
// -------------------------------------------------------


// I think this converts a float stored in a vec2 into an actual float
// (Probably some sort of packing)
float V2F16(vec2 v) {
    return v.y * float(0.0039215689) + v.x;
}

vec3 linear_to_srgb(vec4 c) {
    return pow(c.rgb, vec3(2.2));
}

vec3 sgrb_to_linear(vec4 c) {
    return pow(c.rgb, vec3(1.0 / 2.2));
}

// Note: alpha-channel represents occlusion/visibility where 0 is occluded and 1 is non-occluded
vec4 raymarch(vec2 origin, vec2 direction, float max_length) {
    float df = 0.0;
    float ray_distance = 0.0;
    float scale = length(scene);
    for (float i = 0.0; i < max_length; i ++;) {
        vec2 ray = (origin + direction * ray_distance) * (1.0 / screen_dimensions);
        df = V2F16(texture(dist_field, ray).rg);
        ray_distance += df * scale;
        ray_origin += ray_dir * df * scale * TEXEL;

        // Ray out-of-bounds or has travelled too far
        if (ray_distance >= max_length || floor(ray) != vec2(0.0)) break;

        //if (df < EPSILON && ray_distance < EPSILON && cascade_index != 0.0) {
        //    return vec4(0.0);
        //}

        // Ray hit => Return the colour of the hit part of the scene
        if (df <= EPSILON) {
            return vec4(srgb(texture(scene, ray).rgb), 0.0);
        }
    }

    // No hit => no occlusion
    return vec4(0.0, 0.0, 0.0, 1.0); 
}

vec4 merge(vec4 radiance, float neighbour_index, vec2 dir_block_size, vec2 dir_block_index) {
    // Do not merge with the prev cascade if the ray has hit an occluder or we are at the first (highest) cascade
   if (radiance.a == 0.0 || cascade_index >= num_cascades - 1.0) {
        return vec4(radiance.rgb, 1.0 - radiance.a);
   }

   float prev_angular = pow(2.0, floor(cascade_index + 1.0));   // TODO: uniform
   vec2 prev_dir_block_size = floor(cascade_dimensions / prev_angular); //TODO: uniform
   vec2 interpolation_point = vec2(mod(neighbour_index, prev_angular), floor(neighbour_index / prev_angular)) * dir_block_size;
   interpolation_point += clamp(0.5 * dir_block_index + 0.25, vec2(0.5), prev_dir_block_size - 0.5);
   return radiance + texture(next_cascade, interpolation_point * (1.0 / cascade_dimensions));
}

void main(void) {
    const vec2 coord = floor(tex_coord * cascade_dimensions);
    const float angular_res_sqr = pow(2.0, cascade_index);    //"angular" (number of rays), TODO: turn into uniform

    const vec2 dir_block_size = cascade_dimensions / angular_res_sqr;  //"extent", TODO: uniform
    const vec2 dir_block_index = mod(coord, dir_block_size); //"probe.xy"
    const vec2 probe_pos = floor(tex_coord * angular_res_sqr); //"probe.zw"

    const float ray_offset = (c0_interval_length * (1.0 - pow(4.0, cascade_index))) / (1.0 - 4.0);  //"interval", Some magic math, TODO: uniform
    const vec2 probe_spacing = vec2(c0_probe_density * angular_res_sqr); // "linear", TODO: uniform
    const float interval_length = c0_interval_length * pow(4.0, cascade_index); //"limit", TODO: uniform

    const vec2 origin = (dir_block_index + 0.5) * probe_spacing;
    const float angular = angular_res_sqr * angular_res_sqr * 4.0;  // TODO: uniform
    const float index = (probe_pos.x + (probe_pos.y * angular_res_sqr)) * 4.0;

    color = vec4(0.0);
    for (float i = 0.0; i < 4.0; i++;) {
        const float preavg_ray_index = index + i;  //"preavg"
        const float ray_angle = (preavg_ray_index + 0.5) * (2.0 * 3.14159266 / angular);   //"theta"

        const vec2 ray_dir = vec2(cos(angle), -sin(angle));   //"delta"
        const vec2 ray_origin = origin + ray_dir * ray_offset;

        vec4 radiance = raymarch(ray_origin, ray_dir, interval_length);
        color += merge(radiance, preavg_ray_index, dir_block_size, dir_block_index) * 0.25;
    }

    // This is the colour that should be outputted to screen
    if (cascade_index == 0.0) {
        color = vec4(sgrb_to_linear(color, 1.0));
    }
}