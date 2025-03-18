// neun und neunzig metaballs or spicey metaballs
// by Ingemar Ragnemalm 2018:
// A simple 2D metaball hack.

#version 150

out vec4 outColor;
in vec2 screenCoord;

uniform vec2 balls[99];
uniform float ballsize[99];

void main(void)
{
	float sum = 0;
	for (int i = 0; i < 99; i++)
	{
//		sum += ballsize[i]*1.0/((screenCoord.x - balls[i].x)*(screenCoord.x - balls[i].x) + (screenCoord.y - balls[i].y)*(screenCoord.y - balls[i].y)); // sum > 99 Linear = square distance, not very gooey
		sum += ballsize[i]*1.0/sqrt((screenCoord.x - balls[i].x)*(screenCoord.x - balls[i].x) + (screenCoord.y - balls[i].y)*(screenCoord.y - balls[i].y)); // sum > 18 Square root = actual distance
//		sum += ballsize[i]*1.0/sqrt(sqrt((screenCoord.x - balls[i].x)*(screenCoord.x - balls[i].x) + (screenCoord.y - balls[i].y)*(screenCoord.y - balls[i].y))); // sum > 9 Root of distance, pretty good but the points are too distant in this demo
	}
	if (sum > 18)
		outColor = vec4(1, 0, 0 ,1);
	else
		outColor = vec4(0, 0, 1.0 ,1);
}


// Without sqrt, use limit 99
