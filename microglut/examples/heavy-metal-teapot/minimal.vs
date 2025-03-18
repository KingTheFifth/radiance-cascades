#version 410 core

// Pass-through
in vec3 in_Position;
in vec2 in_TexCoord;
in vec3 in_Normal;
out vec4 vPosition;
out vec2 vTexCoord;
out vec3 vNormal;
uniform mat4 m;

void main()
{
    vPosition = m * vec4(in_Position, 1.0);
    vTexCoord = in_TexCoord;
    vNormal = mat3(m) * in_Normal;
}
