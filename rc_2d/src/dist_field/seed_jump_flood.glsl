#version 450

uniform sampler2D tex;
in vec2 tex_coord;
out vec4 color;

vec2 f16v2(float f) {
    return vec2(floor(f * 255.0) * float(0.0039215689), fract(f * 255.0));
}

void main() {
    vec4 scene = texture(tex, tex_coord);
    color = vec4(f16v2(tex_coord.x * scene.a), f16v2(tex_coord.y * scene.a));
}