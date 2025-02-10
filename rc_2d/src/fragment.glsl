#version 450

in vec2 v_tex_coord;
out vec4 color;

const float EPSILON = 0.0001;

// RC params ---------------------------------------------
uniform sampler2D scene;
uniform sampler2D dist_field;
uniform vec2 screen_dimensions;
uniform vec2 cascade_dimensions;
uniform float num_cascades;
uniform float cascade_index;          // Current cascade

// These get scaled for each cascade: 0.25x density and 4x rays for each cascade in comparison to previous one
// (Other factors are possible but this keeps the cascade dimensions equal for all cacades)
uniform float c0_probe_density;     // As power of 2
uniform float c0_interval_length;


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
vec4 raymarch(vec2 origin, vec2 direction, float offset, float max_length) {
    vec2 ray = (origin + direction * offset) * TEXEL;

    float df = 0.0;
    float ray_distance = 0.0;
    float scale = length(scene);
    for (float i = 0.0; i < max_length; i ++;) {
        // TODO: TEXEL
        df = V2F16(texture(dist_field, ray).rg);
        ray_distance += df * scale;
        ray += direction * df * scale * TEXEL;

        // Ray out-of-bounds or has travelled too far
        if (ray_distance >= max_length || floor(ray) != vec2(0.0)) break;

        if (df < EPSILON && ray_distance < EPSILON && cascade_index != 0.0) {
            return vec4(0.0);
        }

        if (df < EPSILON) {
            return vec4(srgb(texture(scene, ray).rgb), 0.0);
        }
    }

    // No hit => no occlusion
    return vec4(0.0, 0.0, 0.0, 1.0); 
}

vec4 merge(vec4 radiance, float neighbour_index) {
   if (radiance.a == 0.0 || cascade_index >= num_cascades - 1.0) {
        return vec4(radiance.rgb, 1.0 - radiance.a);
   }

    // TODO: all the values that need to go in here
   const float angular_res_next = pow(2.0, cascade_index + 1.0);
   const vec2 dir_block_size_next = dir_block_size * 0.5;
   const vec2 dir_block_index_next = vec2(mod(neighbour_index, angular_res_next), floor(neighbour_index / angular_res_next));
   const vec2 interp_uv_next = dir_block_index * 0.5;
   interp_uv_next = max(vec2(1.0), min(interp_uv_next, dir_block_size_next - 1.0)); // Clamp

   const vec2 probe_uv_next = dir_block_index_next + interp_uv_next + 0.25;
   const vec4 interp_radiance = texture(TODO, probe_uv_next * (1.0 / cascade_dimensions));

    return radiance + interp_radiance;
}

void main(void) {
    const vec2 coord = floor(v_tex_coord * cascade_dimensions);
    const float angular_resolution = pow(2.0, cascade_index);    // (number of rays)
    const vec2 probe_spacing = vec2(c0_probe_density * angular_resolution);
    const vec2 direction_block_size = cascade_dimensions / angular_resolution;
    const vec2 direction_block_index = mod(floor(coord), direction_block_size);
    const vec2 probe_pos = floor(v_tex_coord * angular_resolution);
    const float pre_avg_index = probe_pos.x + (angular_resolution * probe_pos.y);
    const float ray_offset = (c0_interval_length * (1.0 - pow(4.0, cascade_index))) / (1.0 - 4.0);  // Some magic math
    const float interval_length = c0_interval_length * pow(4.0, cascade_index);

    const vec2 origin = (dir_block_index + 0.5) * probe_spacing;
    const float pre_avg_index_22222222222 =  pre_avg_index * 4.0;
    const float theta_scalar = 2 * 3.14159266 / (angular_resolution * 4.0);

    color = vec4(0.0);
    for (float i = 0.0; i < 4.0; i++;) {
        const float index = pre_avg_index_22222222222 + i;
        const float angle = (index + 0.5) * theta_scalar;
        const vec2 direction = vec2(cos(angle), -sin(angle));
        vec4 radiance = raymarch(origin, angle, ray_offset, interval_length);
        color += merge(radiance, index) * 0.25;
    }

    // This is the colour that should be outputted to screen
    if (cascade_index == 0.0) {
        color = vec4(sgrb_to_linear(color, 1.0));
    }
}