#version 450

in vec2 tex_coord;
uniform vec2 screen_dimensions;
uniform sampler2D tex;
out vec4 color;

float v2f16(vec2 v) {
    return v.y * float(0.0039215689) + v.x;
}

vec2 f16v2(float f) {
    return vec2(floor(f * 255.0) * float(0.0039215689), fract(f * 255.0));
}

void main() {
    vec4 jfuv = texture(tex, tex_coord);
    vec2 jumpflood = vec2(v2f16(jfuv.rg), v2f16(jfuv.ba));
    float dist = distance(tex_coord * screen_dimensions, jumpflood * screen_dimensions);
    color = vec4(f16v2(dist / length(screen_dimensions)), 0.0, 1.0);
}