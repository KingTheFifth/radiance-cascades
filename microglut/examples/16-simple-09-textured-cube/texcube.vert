#version 150

// Simple example with transformation matrices (too many, really,
// they should be multiplied on the CPU)

in  vec3 in_Position;
in  vec2 in_Texcoord;
uniform mat4 rotationMatrix;
uniform mat4 rotationMatrix2;
uniform mat4 translationMatrix;
uniform mat4 projMatrix;
out  vec2 texcoord;

void main(void)
{
	gl_Position = projMatrix * translationMatrix * rotationMatrix * rotationMatrix2 * vec4(in_Position, 1.0);
	texcoord = in_Texcoord;
}
