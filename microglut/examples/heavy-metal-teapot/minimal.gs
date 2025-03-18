#version 410 core

layout(triangles) in;
layout(triangle_strip, max_vertices = 3) out;
//layout(line_strip, max_vertices = 3) out;
in vec2 teTexCoord[3];
in vec3 teNormal[3];
out vec2 gsTexCoord;
out vec3 gsNormal;
out vec4 vv;
uniform sampler2D tex;

// Recalc normals!

uniform float disp;
uniform int texon;

void main()
{
	vec4 modpos[3];
	float t;
	const float kBase = 1.0;
	
    t = kBase-texture(tex, teTexCoord[0]).x;
	modpos[0] = gl_in[0].gl_Position + disp*vec4(normalize(teNormal[0]) * t, 1.0);
    t = kBase-texture(tex, teTexCoord[1]).x;
	modpos[1] = gl_in[1].gl_Position + disp*vec4(normalize(teNormal[1]) * t, 1.0);
    t = kBase-texture(tex, teTexCoord[2]).x;
	modpos[2] = gl_in[2].gl_Position + disp*vec4(normalize(teNormal[2]) * t, 1.0);
	vec3 v1 = vec3(modpos[1] - modpos[0]);
	vec3 v2 = vec3(modpos[2] - modpos[0]);
	vec3 n = normalize(cross(v2, v1));
//	gsNormal = n;
	
    t = kBase-texture(tex, teTexCoord[0]).x;
    gl_Position = gl_in[0].gl_Position + disp*vec4(normalize(teNormal[0]) * t, 1.0);
    gsTexCoord = teTexCoord[0];
//    gsNormal = teNormal[0];
	gsNormal = n;
    EmitVertex();
    t = kBase-texture(tex, teTexCoord[1]).x;
    gl_Position = gl_in[1].gl_Position + disp*vec4(normalize(teNormal[1]) * t, 1.0);
    gsTexCoord = teTexCoord[1];
//    gsNormal = teNormal[1];
	gsNormal = n;
    EmitVertex();
    t = kBase-texture(tex, teTexCoord[2]).x;
    gl_Position = gl_in[2].gl_Position + disp*vec4(normalize(teNormal[2]) * t, 1.0);
    gsTexCoord = teTexCoord[2];
//    gsNormal = teNormal[2];
	gsNormal = n;
    EmitVertex();

    EndPrimitive();
}

