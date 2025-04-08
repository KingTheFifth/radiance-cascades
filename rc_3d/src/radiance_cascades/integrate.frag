#version 450

in vec2 tex_coord;
out vec4 color;

uniform sampler2D cascade;
uniform sampler2D scene_normal;
uniform sampler2D scene_albedo;
uniform sampler2D scene_emissive;
uniform sampler2D depth_tex;
uniform float cascade_index;

uniform float ambient;

layout(std430) readonly buffer RCConstants {
    vec2 c0_resolution;
    float num_cascades;
    float c0_probe_spacing;
    float c0_interval_length;
};

layout(std430) readonly buffer SceneMatrices {
    mat4 world_to_view;
    mat4 world_to_view_inv;
    mat4 perspective;
    mat4 perspective_inv;
    vec2 screen_res;
    vec2 screen_res_inv;
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

float screen_depth_to_view_depth(float depth) {
    // NOTE: These calculations depend on the projection matrix
    float remapped_depth = depth * 2.0 - 1.0;
    return - z_near * z_far / (z_far + remapped_depth * (z_near - z_far));
}

float linearize_depth(float depth) {
    return -1.0 * screen_depth_to_view_depth(depth) / (z_far - z_near);
}

void main() {
    const float probe_spacing = c0_probe_spacing * pow(2.0, cascade_index);
    const vec2 block_coord = gl_FragCoord.xy / probe_spacing;
    const ivec2 block_coord_base = ivec2(floor(block_coord));
    const vec2 ratio = fract(block_coord);
    const vec4 bilinear_weights = vec4(
        (1.0 - ratio.x) * (1.0 - ratio.y),
        ratio.x * (1.0 - ratio.y),
        (1.0 - ratio.x) * ratio.y,
        ratio.x * ratio.y
    );

    const ivec2 offsets[4] = {ivec2(0, 0), ivec2(1, 0), ivec2(0, 1), ivec2(1, 1)};
    const ivec2 probe_coords[4] = {
        block_coord_base + offsets[0],
        block_coord_base + offsets[1],
        block_coord_base + offsets[2],
        block_coord_base + offsets[3]
    };
    const ivec2 probe_screen_coords[4] = {
        probe_coords[0] * int(probe_spacing),
        probe_coords[1] * int(probe_spacing),
        probe_coords[2] * int(probe_spacing),
        probe_coords[3] * int(probe_spacing)
    };

    const float depth = linearize_depth(texelFetch(depth_tex, ivec2(gl_FragCoord.xy), 0).x);
    const vec3 normal = octahedral_decode(texelFetch(scene_normal, ivec2(gl_FragCoord.xy), 0).xy);
    const vec4 depths = vec4(
        linearize_depth(texelFetch(depth_tex, probe_screen_coords[0], 0).x),
        linearize_depth(texelFetch(depth_tex, probe_screen_coords[1], 0).x),
        linearize_depth(texelFetch(depth_tex, probe_screen_coords[2], 0).x),
        linearize_depth(texelFetch(depth_tex, probe_screen_coords[3], 0).x)
    );
    const vec4 normals = vec4(
        clamp(dot(octahedral_decode(texelFetch(scene_normal, probe_screen_coords[0], 0).xy), normal) * 1.5, 0.0, 1.0),
        clamp(dot(octahedral_decode(texelFetch(scene_normal, probe_screen_coords[1], 0).xy), normal) * 1.5, 0.0, 1.0),
        clamp(dot(octahedral_decode(texelFetch(scene_normal, probe_screen_coords[2], 0).xy), normal) * 1.5, 0.0, 1.0),
        clamp(dot(octahedral_decode(texelFetch(scene_normal, probe_screen_coords[3], 0).xy), normal) * 1.5, 0.0, 1.0)
    );

    // Edge detection
    float min_depth = min(min(depths.x, depths.y), min(depths.z, depths.w));
    float max_depth = max(max(depths.x, depths.y), max(depths.z, depths.w));
    float depth_diff = max_depth - min_depth;
    float avg_depth = dot(depths, vec4(0.25));
    bool depth_edge = (depth_diff / avg_depth) > 0.05;

    vec4 probe_weights = bilinear_weights * normals;
    if (depth_edge) {
        // Use depth probe_weights bilaterally when on a depth edge
        vec4 dd = abs(depths - vec4(depth));
        probe_weights *= vec4(1.0) / (dd + vec4(0.0001));
    }
    probe_weights /= dot(probe_weights, vec4(1.0));

    float altitudinal_dirs = 4.0 * pow(2.0, cascade_index);
    float altitudinal_dirs_inv = 1.0 / altitudinal_dirs;
    float azimuthal_dirs = 4.0 * pow(2.0, cascade_index);
    float azimuthal_dirs_inv = 1.0 / azimuthal_dirs;
    ivec2 probe_count = ivec2(floor(screen_res / probe_spacing));
    vec2 scale_bias = vec2(azimuthal_dirs_inv, altitudinal_dirs_inv);

    vec3 radiance = vec3(0.0);
    float total_cone_weight = 0.0;
    for (float alt = 0.0; alt < altitudinal_dirs; alt += 1.0) {
        const float altitude = (alt + 0.5) * (3.14159265 * altitudinal_dirs_inv);
        const float cos_altitude = cos(altitude); 
        const float sin_altitude = sin(altitude); 

        for (float azi = 0.0; azi < azimuthal_dirs; azi++) {
            const vec2 cone_coord = vec2(tex_coord * scale_bias + vec2(azi, alt) * scale_bias);
            const vec3 cone_radiance = texture(cascade, cone_coord).rgb;

            const ivec2 dir_block_start = probe_count * ivec2(azi, alt);
            vec4 r = texelFetch(cascade, dir_block_start + probe_coords[0], 0) * probe_weights[0];
            r += texelFetch(cascade, dir_block_start + probe_coords[1], 0) * probe_weights[1];
            r += texelFetch(cascade, dir_block_start + probe_coords[2], 0) * probe_weights[2];
            r += texelFetch(cascade, dir_block_start + probe_coords[3], 0) * probe_weights[3];

            const float azimuth = (azi + 0.5) * (2.0 * 3.14159265 * azimuthal_dirs_inv);
            const vec3 cone_direction = normalize(vec3(
                cos(azimuth) * sin_altitude,
                cos_altitude,
                sin(azimuth) * sin_altitude
            ));

            float cone_weight = max(0.0, dot(cone_direction, normal));
            radiance += r.rgb * cone_weight;
            total_cone_weight += cone_weight;
        }
    }
    radiance = (total_cone_weight > 0.0) ? radiance / total_cone_weight : vec3(0.0);

    const vec4 albedo = texture(scene_albedo, tex_coord);
    const vec3 emissive = texture(scene_emissive, tex_coord).rgb;
    color = vec4(srgb_to_linear(albedo.rgb * (radiance + ambient) + emissive), albedo.a);
}