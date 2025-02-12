#version 450

in vec2 tex_coord;
out vec4 color;

void main() {
    float x_sqr = (tex_coord.x - 0.5) * (tex_coord.x - 0.5);
    float y_sqr = (tex_coord.y - 0.5) * (tex_coord.y - 0.5);
    if (x_sqr + y_sqr <= 0.01) {
        color = vec4(1.0, 0.0, 1.0, 1.0);
    }
    else {
        color = vec4(0.0);
    }
}