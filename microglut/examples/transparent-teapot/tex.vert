#version 150

in vec3 inPosition;
in vec3 inNormal;
in vec2 inTexCoord;
out vec2 texCoord;
out vec3 normal;

uniform mat4 projMatrix;
uniform mat4 lookAtMatrix;
uniform mat4 rotationMatrix;

void main(void)
{
	texCoord = inTexCoord;
	normal = mat3(lookAtMatrix * rotationMatrix) * inNormal;
	
	gl_Position = projMatrix * lookAtMatrix * rotationMatrix * vec4(inPosition, 1.0);
}
