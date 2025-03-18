#version 410 core

out vec4 out_Color;
in vec2 gsTexCoord;
in vec3 gsNormal;
uniform sampler2D tex;
uniform sampler2D flower;
uniform int texon;

void main(void)
{
	float shade = abs(gsNormal.z);
	out_Color = vec4(gsTexCoord.s, gsTexCoord.t, 0.0, 1.0);
	if (texon == 1)
		out_Color = texture(tex, gsTexCoord) * (shade);
	else
	if (texon == 2)
		out_Color = texture(flower, gsTexCoord) * (shade);
	else
		out_Color = vec4(shade, shade, shade, 1.0);
}

