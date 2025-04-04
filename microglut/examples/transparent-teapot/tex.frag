#version 150

out vec4 outColor;
in vec2 texCoord;
in vec3 normal;
uniform sampler2D tex;

void main(void)
{
	const vec3 light = vec3(0.58, 0.58, 0.58);
	float shade = dot(normal, light);

// Texture * shade (don't shade alpha!):
	vec4 p = texture(tex, texCoord);
	outColor = vec4(p.r*shade, p.g*shade, p.b*shade, p.a);
}
