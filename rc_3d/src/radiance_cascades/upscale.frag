#version 450

uniform sampler2D full_res_tex;
uniform sampler2D half_res_tex;
uniform sampler2D depth_tex;
uniform sampler2D normal_tex;

in vec2 tex_coord;
out vec4 color;

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

float screen_depth_to_view_depth(float depth) {
    // NOTE: These calculations depend on the projection matrix
    float remapped_depth = depth * 2.0 - 1.0;
    return - z_near * z_far / (z_far + remapped_depth * (z_near - z_far));
}

float linearize_depth(float depth) {
    return -1.0 * screen_depth_to_view_depth(depth) / (z_far - z_near);
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

#define USE_TEXEL_FETCH

void main() {
    int scale = 4;
    float scale_inv = 1.0 / float(scale);
    vec2 low_res_dimensions = textureSize(full_res_tex, 0) * scale_inv;
    vec2 low_res_coord = gl_FragCoord.xy * scale_inv;
    ivec2 low_res_coord_base = ivec2(floor(low_res_coord));
    vec2 low_res_coord_frac = fract(low_res_coord);
    vec4 bilinear_weights = vec4(
        (1.0 - low_res_coord_frac.x) * (1.0 - low_res_coord_frac.y),
        low_res_coord_frac.x * (1.0 - low_res_coord_frac.y),
        (1.0 - low_res_coord_frac.x) * low_res_coord_frac.y,
        low_res_coord_frac.x * low_res_coord_frac.y
    );

    ivec2 offsets[4] = {ivec2(0.0, 0.0), ivec2(1.0, 0.0), ivec2(0.0, 1.0), ivec2(1.0, 1.0)};
    ivec2 low_res_coords[4] = {
        (low_res_coord_base + offsets[0]),
        (low_res_coord_base + offsets[1]),
        (low_res_coord_base + offsets[2]),
        (low_res_coord_base + offsets[3])
    };
    vec2 low_res_uvs[4] = {
        (vec2(low_res_coords[0]) * float(scale) + 0.5 * float(scale)) / textureSize(full_res_tex, 0),
        (vec2(low_res_coords[1]) * float(scale) + 0.5 * float(scale)) / textureSize(full_res_tex, 0),
        (vec2(low_res_coords[2]) * float(scale) + 0.5 * float(scale)) / textureSize(full_res_tex, 0),
        (vec2(low_res_coords[3]) * float(scale) + 0.5 * float(scale)) / textureSize(full_res_tex, 0)
    };

    #ifdef USE_TEXEL_FETCH
    float linear_depth = linearize_depth(texelFetch(depth_tex, ivec2(gl_FragCoord.xy), 0).r);
    vec4 depths = vec4(
        linearize_depth(texelFetch(depth_tex, low_res_coords[0] * scale, 0).r),
        linearize_depth(texelFetch(depth_tex, low_res_coords[1] * scale, 0).r),
        linearize_depth(texelFetch(depth_tex, low_res_coords[2] * scale, 0).r),
        linearize_depth(texelFetch(depth_tex, low_res_coords[3] * scale, 0).r)
    );

    vec3 normal = octahedral_decode(texelFetch(normal_tex, ivec2(gl_FragCoord.xy), 0).xy);
    vec4 normals = {
        clamp(dot(octahedral_decode(texelFetch(normal_tex, low_res_coords[0] * scale, 0).xy), normal) * 1.5, 0.0, 1.0),
        clamp(dot(octahedral_decode(texelFetch(normal_tex, low_res_coords[1] * scale, 0).xy), normal) * 1.5, 0.0, 1.0),
        clamp(dot(octahedral_decode(texelFetch(normal_tex, low_res_coords[2] * scale, 0).xy), normal) * 1.5, 0.0, 1.0),
        clamp(dot(octahedral_decode(texelFetch(normal_tex, low_res_coords[3] * scale, 0).xy), normal) * 1.5, 0.0, 1.0)
    };

    #else
    float linear_depth = linearize_depth(texture(depth_tex, tex_coord).r);
    vec4 depths = vec4(
        linearize_depth(texture(depth_tex, low_res_uvs[0]).r),
        linearize_depth(texture(depth_tex, low_res_uvs[1]).r),
        linearize_depth(texture(depth_tex, low_res_uvs[2]).r),
        linearize_depth(texture(depth_tex, low_res_uvs[3]).r)
    );

    vec3 normal = octahedral_decode(texture(normal_tex, tex_coord).xy);
    vec4 normals = {
        clamp(dot(octahedral_decode(texture(normal_tex, low_res_uvs[0]).xy), normal) * 1.5, 0.0, 1.0),
        clamp(dot(octahedral_decode(texture(normal_tex, low_res_uvs[1]).xy), normal) * 1.5, 0.0, 1.0),
        clamp(dot(octahedral_decode(texture(normal_tex, low_res_uvs[2]).xy), normal) * 1.5, 0.0, 1.0),
        clamp(dot(octahedral_decode(texture(normal_tex, low_res_uvs[3]).xy), normal) * 1.5, 0.0, 1.0)
    };
    #endif

    // Edge detection
    float min_depth = min(min(depths.x, depths.y), min(depths.z, depths.w));
    float max_depth = max(max(depths.x, depths.y), max(depths.z, depths.w));
    float depth_diff = max_depth - min_depth;
    float avg_depth = dot(depths, vec4(0.25));
    bool depth_edge = (depth_diff / avg_depth) > 0.1;

    vec4 weights = bilinear_weights * normals;
    if (depth_edge) {
        // Use depth weights bilaterally when on a depth edge
        vec4 dd = abs(depths - vec4(linear_depth));
        weights *= vec4(1.0) / (dd + vec4(0.0001));
    }
    weights /= dot(weights, vec4(1.0));

    vec4 accumulated_color = vec4(0.0);
    for (int i = 0; i < 4; i++) {
        #ifdef USE_TEXEL_FETCH
        accumulated_color += texelFetch(full_res_tex, low_res_coords[i] * scale, 0) * weights[i];
        #else
        accumulated_color += texture(full_res_tex, low_res_uvs[i]) * weights[i];
        #endif
    }

    #ifdef USE_TEXEL_FETCH
    color = (tex_coord.x < 0.5) ? texelFetch(full_res_tex, low_res_coord_base * scale, 0) : vec4(accumulated_color.rgb, 1.0);
    #else
    color = (tex_coord.x < 0.5) ? texture(full_res_tex, low_res_uvs[0]) : vec4(accumulated_color.rgb, 1.0);
    #endif

    color = (abs(tex_coord.x - 0.5) < 0.001) ? (1.0 - color) : color;
}