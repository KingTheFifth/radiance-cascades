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

vec4 merge(vec4 radiance, float dir_index, vec2 coord_within_block) {
    // Do not merge with the prev cascade if the ray has hit an occluder or we are at the first (highest) cascade
   if (radiance.a == 0.0 || cascade_index >= num_cascades - 1.0) {
        return vec4(radiance.rgb, 1.0 - radiance.a);
   }

   float prev_num_dirs_sqrt = pow(2.0, floor(cascade_index + 1.0));   // TODO: uniform
   vec2 prev_dir_block_size = floor(cascade_dimensions / prev_num_dirs_sqrt); //TODO: uniform

   vec2 interpolation_point = vec2(mod(dir_index, prev_num_dirs_sqrt), floor(dir_index / prev_num_dirs_sqrt)) * prev_dir_block_size;
   interpolation_point += clamp(0.5 * coord_within_block + 0.25, vec2(0.5), prev_dir_block_size - 0.5);

   return radiance + texture(prev_cascade, interpolation_point * (1.0 / cascade_dimensions));
}

vec4 merge_nearest(vec4 radiance, float dir_index, vec2 coord_within_block) {
   if (radiance.a == 0.0 || cascade_index >= num_cascades - 1.0) {
        return vec4(radiance.rgb, 1.0 - radiance.a);
   }

   const float prev_num_dirs_sqrt = pow(2.0, floor(cascade_index + 1.0));
   const vec2 prev_dir_block_size = floor(cascade_dimensions / prev_num_dirs_sqrt);
   vec2 interpolation_point = vec2(mod(dir_index, prev_num_dirs_sqrt), floor(dir_index / prev_num_dirs_sqrt)) * prev_dir_block_size;
   interpolation_point += clamp(coord_within_block + 0.5, vec2(0.5), prev_dir_block_size - 0.5);
   return texture(prev_cascade, interpolation_point * (1.0 / cascade_dimensions));
}

void get_bilinear_probes(vec2 coord, out vec2 coords[4]) {
    vec2 prev_cascade_coord = floor(coord * 0.5 - 0.5);
    coords[0] = prev_cascade_coord + vec2(0.0, 0.0);
    coords[1] = prev_cascade_coord + vec2(1.0, 0.0);
    coords[2] = prev_cascade_coord + vec2(0.0, 1.0);
    coords[3] = prev_cascade_coord + vec2(1.0, 1.0);
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
    // Direction index i corresponds to the angle 2*pi/num_dirs * i
    const vec2 origin = (coord_within_block + 0.5) * probe_spacing;
    const float num_dirs = num_dirs_sqrt * num_dirs_sqrt * 4.0;  // TODO: uniform
    const float dir_index = (dir_block_index.x + (dir_block_index.y * num_dirs_sqrt)) * 4.0;

    // Bilinear interpolation fix
    vec2 prev_probe_spacing = vec2(c0_probe_spacing * pow(2.0, cascade_index + 1.0));
    vec2 prev_cascade_coords[4];
    get_bilinear_probes(coord_within_block, prev_cascade_coords);
    vec2 prev_cascade_coord = floor(coord_within_block * 0.5 - 0.5);
    vec2 man_wtf = floor(prev_cascade_coord * 2.0 + 1.0);
    vec2 weight = vec2(0.25) + (coord_within_block - man_wtf) * 0.5;

    // Cast 4 rays and average together into one ray to save memory and calculations
    const float TAU = 2.0 * 3.14159266;
    color = vec4(0.0);
    for (float i = 0.0; i < 4.0; i++) {
        const float preavg_dir_index = dir_index + float(i);  //"preavg"
        const float ray_angle = (preavg_dir_index + 0.5) * (TAU / num_dirs);   //"theta"
        const float ray_angle_Nm1 = (floor(preavg_dir_index / 4.0) + 0.5) * (TAU / (num_dirs / 4.0));

        const vec2 ray_dir = vec2(cos(ray_angle), -sin(ray_angle));   //"delta"
        const vec2 ray_dir_Nm1 = vec2(cos(ray_angle_Nm1), -sin(ray_angle_Nm1));
        //const vec2 ray_origin = origin + ray_dir * interval_start;
        const vec2 ray_origin = origin + ray_dir_Nm1 * interval_start;

        vec4 radiance = raymarch(ray_origin, ray_dir, interval_length);

        // Bilinear fix
        vec4 samples[4];
        for (float j = 0.0; j < 4.0; j++) {
            const vec2 ray_origin_N1 = (prev_cascade_coords[int(j)] + 0.5) * prev_probe_spacing;
            const vec2 ray_end = ray_origin_N1 + ray_dir * (interval_start + interval_length);
            const vec4 RADIANCE = raymarch(ray_origin, normalize(ray_end - ray_origin), length(ray_end - ray_origin));
            samples[int(j)] = merge_nearest(RADIANCE, preavg_dir_index, prev_cascade_coords[int(j)]);
        }

        const vec4 top = mix(samples[0], samples[1], weight.x);
        const vec4 bot = mix(samples[2], samples[3], weight.x);
        color += mix(top, bot, weight.y) * 0.25;

        // Merge the previous cascade (of higher index) into this one
        //color += merge(radiance, preavg_dir_index, coord_within_block) * 0.25;
    }

    // After merging all higher cascades into cascade 0,
    // cascade 0 contains the final colour to output for this fragment
    if (cascade_index == 0.0) {
        color = texture(scene_albedo, tex_coord) + vec4(srgb_to_linear(color.rgb), 1.0);
    }
}