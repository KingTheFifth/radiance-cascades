#version 150

in  vec3 in_Position;
// NY
uniform mat4 myMatrix;

void main(void)
{
//	gl_Position = vec4(in_Position, 1.0);
	gl_Position = myMatrix * vec4(in_Position, 1.0);
}
