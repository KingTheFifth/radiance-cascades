#version 450

in vec3 position;

uniform mat4 world_to_view;
uniform mat4 projection;

void main() {
    gl_Position = projection * world_to_view * vec4(position, 1.0);
}