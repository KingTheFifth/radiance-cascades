#version 150

in  vec3 inPosition;
//in  vec3 inColor;
//in  vec3 inNormal;
in vec2 inTexCoord;

//out vec3 exColor; // Gouraud
//out vec3 exNormal; // Phong
out vec2 texCoord;

void main(void)
{
	texCoord = inTexCoord;
	
	gl_Position = vec4(inPosition, 1.0);
}
