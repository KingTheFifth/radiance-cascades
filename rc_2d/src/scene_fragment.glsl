#version 450

in vec2 tex_coord;
out vec4 color;

void main() {
    float x_sqr = (tex_coord.x - 0.5) * (tex_coord.x - 0.5);
    float y_sqr = (tex_coord.y - 0.5) * (tex_coord.y - 0.5);
    float x_sqr_2 = (tex_coord.x - 0.7) * (tex_coord.x - 0.7);
    float y_sqr_2 = (tex_coord.y - 0.2) * (tex_coord.y - 0.2);
    float x_sqr_3 = (tex_coord.x - 0.35) * (tex_coord.x - 0.35);
    float y_sqr_3 = (tex_coord.y - 0.325) * (tex_coord.y - 0.325);
    float x_sqr_4 = (tex_coord.x - 0.35) * (tex_coord.x - 0.35);
    float y_sqr_4 = (tex_coord.y - 0.3) * (tex_coord.y - 0.3);
    float x_sqr_5 = (tex_coord.x - 0.65) * (tex_coord.x - 0.65);
    float y_sqr_5 = (tex_coord.y - 0.6) * (tex_coord.y - 0.6);
    if (x_sqr + y_sqr <= 0.01) {
        color = vec4(1.0, 0.0, 1.0, 1.0);
    }
    else if (x_sqr_2 + y_sqr_2 <= 0.01) {
        color = vec4(0.0, 1.0, 1.0, 1.0);
    }
    else if (x_sqr_3 + y_sqr_3 <= 0.002) {
        color = vec4(0.0, 0.0, 0.0, 1.0);
    }
    else if (x_sqr_4 + y_sqr_4 <= 0.002) {
        color = vec4(0.0, 0.0, 0.0, 1.0);
    }
    else if (x_sqr_5 + y_sqr_5 <= 0.002) {
        color = vec4(1.0, 1.0, 0.0, 1.0);
    }
    else {
        color = vec4(0.0);
    }
    color.rgb *= 1.2;
}