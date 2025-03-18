#version 150

in  vec3 inPosition;
//in  vec3 inColor;
in  vec3 inNormal;

out vec3 exColor; // Gouraud

out vec3 exNormal; // Phong

void main(void)
{
	const vec3 light = vec3(0.58, 0.58, 0.58);
	float shade;
	
// Gouraud:	
	shade = dot(normalize(inNormal), light);
	exColor = vec3(shade); // inColor;
	
	exNormal = inNormal; // Phong
	
	gl_Position = vec4(inPosition, 1.0);
}
