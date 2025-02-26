#version 450

in vec2 tex_coord;

uniform sampler2D depth_tex;
uniform vec2 dimensions;
uniform int level;
uniform int prev_mip_level;
uniform vec2 prev_level_dimensions;

layout(location = 3) out vec2 color;

void main() {
    ivec2 pixel_coord = ivec2(floor(tex_coord * dimensions));

    if (level == 0) {
        float depth = texelFetch(depth_tex, pixel_coord, 0).r;
        color = vec2(depth, depth);
        return;
    }

    ivec2 prev_level_pixel_coord = pixel_coord * 2;
    vec2 depths[4];
    depths[0] = texelFetch(depth_tex, prev_level_pixel_coord, prev_mip_level).rg;
    depths[1] = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(1, 0), prev_mip_level).rg;
    depths[2] = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(0, 1), prev_mip_level).rg;
    depths[3] = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(1, 1), prev_mip_level).rg;

    float min_depth = min(
        min(depths[0].r, depths[1].r),
        min(depths[2].r, depths[3].r)
    );

    float max_depth = max(
        max(depths[0].g, depths[1].g),
        max(depths[2].g, depths[3].g)
    );

    bool include_extra_col_from_prev_level = ((int(prev_level_dimensions.x) & 1) != 0);
    bool include_extra_row_from_prev_level = ((int(prev_level_dimensions.y) & 1) != 0);

    if (include_extra_col_from_prev_level) {
        vec4 extra_col;
        extra_col.rg = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(2, 0), prev_mip_level).rg;
        extra_col.ba = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(2, 1), prev_mip_level).rg;

        if (include_extra_row_from_prev_level) {
            vec2 corner = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(2, 2), prev_mip_level).rg;
            min_depth = min(min_depth, corner.r);
            max_depth = max(max_depth, corner.g);
        }

        min_depth = min(min_depth, min(extra_col.r, extra_col.b));
        max_depth = max(max_depth, max(extra_col.g, extra_col.a));
    }

    if (include_extra_row_from_prev_level) {
        vec4 extra_row;
        extra_row.rg = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(0, 2), prev_mip_level).rg;
        extra_row.ba = texelFetch(depth_tex, prev_level_pixel_coord + ivec2(1, 2), prev_mip_level).rg;

        min_depth = min(min_depth, min(extra_row.r, extra_row.b));
        max_depth = max(max_depth, max(extra_row.g, extra_row.a));
    }

    color = vec2(min_depth, max_depth);
}