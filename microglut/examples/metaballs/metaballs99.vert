#version 150

in  vec3 inPosition;
in vec2 inTexCoord;
out vec2 screenCoord;

void main(void)
{
	gl_Position = vec4(inPosition, 1.0);
	screenCoord = vec2(gl_Position) / gl_Position.w / 2.0 + vec2(0.5);
}
