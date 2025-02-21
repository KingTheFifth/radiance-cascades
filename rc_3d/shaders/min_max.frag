#version 450

in vec2 tex_coord;

uniform sampler2D depth_tex;
uniform vec2 dimensions;
uniform float prev_mip_level;
uniform vec2 prev_level_dimensions;

out vec2 color;

void main() {
    ivec2 pixel_coord = ivec2(floor(tex_coord * dimensions));
    ivec2 prev_level_pixel_coord = pixel_coord * 2;
    vec4 depths;
    depths.x = texelFetch(depth_tex, prev_level_pixel_coord, prev_mip_level).r;
    depths.y = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(1, 0), prev_mip_level).r;
    depths.z = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(0, 1), prev_mip_level).r;
    depths.w = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(1, 1), prev_mip_level).r;

    float min_depth = min(
        min(depths.x, depths.y),
        min(depths.z, depths.w)
    );

    float max_depth = max(
        max(depths.x, depths.y),
        max(depths.z, depths.w)
    );

    bool include_extra_col_from_prev_level = ((prev_level_dimensions.x & 1) != 0);
    bool include_extra_row_from_prev_level = ((prev_level_dimensions.y & 1) != 0);

    if (include_extra_col_from_prev_level) {
        vec2 extra_col;
        extra_col.x = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(2, 0), prev_mip_level).r;
        extra_col.y = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(2, 1), prev_mip_level).r;

        if (include_extra_row_from_prev_level) {
            float corner = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(2, 2), prev_mip_level).r;
            min_depth = min(min_depth, corner);
            max_depth = max(max_depth, corner);
        }

        min_depth = min(min_depth, min(extra_col.x, extra_col.y));
        max_depth = max(max_depth, max(extra_col.x, extra_col.y));
    }

    if (include_extra_row_from_prev_level) {
        vec2 extra_row;
        extra_row.x = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(0, 2), prev_mip_level).r;
        extra_row.y = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(1, 2), prev_mip_level).r;

        min_depth = min(min_depth, min(extra_row.x, extra_row.y));
        max_depth = max(max_depth, max(extra_row.x, extra_row.y));
    }

    color = vec2(min_depth, max_depth);
}