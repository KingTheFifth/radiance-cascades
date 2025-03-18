#version 410 core

// Pass-through
in vec3 in_Position;
out vec3 vPosition;

void main()
{
    vPosition = in_Position;
}

