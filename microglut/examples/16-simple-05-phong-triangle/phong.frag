#version 150

in  vec3 exColor; // Gouraud
out vec4 outColor;

in vec3 exNormal; // Phong

void main(void)
{
// Gouraud
	outColor = vec4(exColor,1.0);
	
// Phong
	const vec3 light = vec3(0.58, 0.58, 0.58);
	float shade;
	
	shade = dot(normalize(exNormal), light);
	outColor = vec4(shade, shade, shade, 1.0); // inColor;
}
