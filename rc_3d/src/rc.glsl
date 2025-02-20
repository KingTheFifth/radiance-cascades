#version 450

in vec2 tex_coord;
out vec4 color;

const float EPSILON = 0.00001;

// RC params ---------------------------------------------
uniform sampler2D scene_albedo;
uniform sampler2D scene_emissive;
uniform sampler2D dist_field;
uniform sampler2D prev_cascade;
uniform vec2 screen_dimensions;
uniform vec2 cascade_dimensions;
uniform float num_cascades;
uniform float cascade_index;          // Current cascade

// These get scaled for each cascade: 0.5x density and 4x rays for each cascade in comparison to previous one
// (Other factors are possible but this keeps the cascade dimensions equal for all cacades)
uniform float c0_probe_spacing;     // As power of 2
uniform float c0_interval_length;
// -------------------------------------------------------


// I think this converts a float stored in a vec2 into an actual float
// (Probably some sort of packing)
float V2F16(vec2 v) {
    return v.y * float(0.0039215689) + v.x;
}

vec3 linear_to_srgb(vec3 c) {
    return pow(c.rgb, vec3(1.6));
}

vec3 srgb_to_linear(vec3 c) {
    return pow(c.rgb, vec3(1.0 / 1.6));
}

// Note: alpha-channel represents occlusion/visibility where 0 is occluded and 1 is non-occluded
vec4 raymarch(vec2 origin, vec2 direction, float max_length) {
    float df = 0.0;
    float ray_distance = 0.0;
    float scale = length(screen_dimensions);
    for (float i = 0.0; i < max_length; i++) {
        vec2 ray = (origin + direction * ray_distance) * (1.0 / screen_dimensions);
        df = V2F16(texture(dist_field, ray).rg);
        ray_distance += df * scale;

        // Ray out-of-bounds or has travelled too far
        if (ray_distance >= max_length || floor(ray) != vec2(0.0)) break;

        //if (df < EPSILON && ray_distance < EPSILON && cascade_index != 0.0) {
        //    return vec4(0.0);
        //}

        // Ray hit => Return the colour of the hit part of the scene
        if (df <= EPSILON) {
            return vec4(linear_to_srgb(texture(scene_emissive, ray).rgb), 0.0);
        }
    }

    // No hit => no occlusion
    return vec4(0.0, 0.0, 0.0, 1.0); 
}

vec4 merge(vec4 radiance, float dir_index, vec2 dir_block_size, vec2 coord_within_block) {
    // Do not merge with the prev cascade if the ray has hit an occluder or we are at the first (highest) cascade
   if (radiance.a == 0.0 || cascade_index >= num_cascades - 1.0) {
        return vec4(radiance.rgb, 1.0 - radiance.a);
   }

   float prev_num_dirs = pow(2.0, floor(cascade_index + 1.0));   // TODO: uniform
   vec2 prev_dir_block_size = floor(cascade_dimensions / prev_num_dirs); //TODO: uniform

   vec2 interpolation_point = vec2(mod(dir_index, prev_num_dirs), floor(dir_index / prev_num_dirs)) * prev_dir_block_size;
   interpolation_point += clamp(0.5 * coord_within_block + 0.25, vec2(0.5), prev_dir_block_size - 0.5);

   return radiance + texture(prev_cascade, interpolation_point * (1.0 / cascade_dimensions));
}

void main(void) {
    const vec2 coord = floor(tex_coord * cascade_dimensions);
    const float num_dirs_sqrt = pow(2.0, cascade_index);    //number of rays, TODO: turn into uniform

    // Partition the output texture into uniform blocks, one for each ray direction,
    // and calculate which block this fragment belongs to
    const vec2 dir_block_size = floor(cascade_dimensions / num_dirs_sqrt);  //TODO: uniform
    const vec2 coord_within_block = mod(coord, dir_block_size);
    const vec2 dir_block_index = floor(coord / dir_block_size);

    // Probe spacing doubles and ray interval length quadruples every cascade
    const vec2 probe_spacing = vec2(c0_probe_spacing * pow(2.0, cascade_index)); //TODO: uniform
    const float interval_length = c0_interval_length * pow(4.0, cascade_index); //TODO: uniform
    const float interval_start = c0_interval_length * ((1.0 - pow(4.0, cascade_index)) / (1.0 - 4.0));  //Geometric sum

    // Calculate probe position and ray direction
    const vec2 origin = (coord_within_block + 0.5) * probe_spacing;
    const float num_dirs = num_dirs_sqrt * num_dirs_sqrt * 4.0;  // TODO: uniform
    const float dir_index = (dir_block_index.x + (dir_block_index.y * num_dirs_sqrt)) * 4.0;

    // Cast 4 rays and average together into one ray to save memory and calculations
    color = vec4(0.0);
    for (float i = 0.0; i < 4.0; i++) {
        const float preavg_dir_index = dir_index + float(i);  //"preavg"
        const float ray_angle = (preavg_dir_index + 0.5) * (2.0 * 3.14159266 / num_dirs);   //"theta"

        const vec2 ray_dir = vec2(cos(ray_angle), -sin(ray_angle));   //"delta"
        const vec2 ray_origin = origin + ray_dir * interval_start;

        vec4 radiance = raymarch(ray_origin, ray_dir, interval_length);

        // Merge the previous cascade (of higher index) into this one
        color += merge(radiance, preavg_dir_index, dir_block_size, coord_within_block) * 0.25;
    }

    // After merging all higher cascades into cascade 0,
    // cascade 0 contains the final colour to output for this fragment
    if (cascade_index == 0.0) {
        color = texture(scene_albedo, tex_coord) + vec4(srgb_to_linear(color.rgb), 1.0);
    }
}