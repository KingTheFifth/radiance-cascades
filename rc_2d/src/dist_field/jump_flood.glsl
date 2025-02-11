#version 450

in vec2 tex_coord;
uniform float jump_dist;
uniform vec2 screen_dimensions;
uniform sampler2D tex;
out vec4 color;

float v2f16(vec2 v) {
    return v.y * float(0.0039215689) + v.x;
}

void main() {
    const vec2 offsets[9] = {
        {-1.0, -1.0},
        {-1.0, 0.0},
        {-1.0, 1.0},
        {0.0, -1.0},
        {0.0, 0.0},
        {0.0, 1.0},
        {1.0, -1.0},
        {1.0, 0.0},
        {1.0, 1.0}
    };

    float closest_dist = 999999.9;
    vec4 closest_data = vec4(0.0);

    for (int i = 0; i < 9; i++;) {
        const vec2 jump = tex_coord + (offsets[i] * vec2(jump_dist / screen_dimensions));
        const vec4 seed = texture(tex, jump);
        const vec2 seed_pos = vec2(v2f16(seed.xy), v2f16(seed.zw));
        const float dist = distance(seed_pos * screen_dimensions, tex_coord * screen_dimensions);

        if (seed_pos != vec2(0.0) && dist <= closest_dist) {
            closest_dist = dist;
            closest_data = seed;
        }
    }

    color = closest_data;
}