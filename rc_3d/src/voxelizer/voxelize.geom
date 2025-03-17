#version 450

layout(triangles) in;
layout(triangle_strip, max_vertices = 3) out;

in vec3 geom_world_pos[];
in vec3 geom_normal[];
in vec4 geom_albedo[];

out vec3 frag_world_pos;
out vec3 frag_normal;
out vec4 frag_albedo;
out float frag_axis;

uniform mat4 projection;
uniform mat4 world_to_view;

void main() {
    const vec3 face_normal = abs(cross(geom_world_pos[1] - geom_world_pos[0], geom_world_pos[2] - geom_world_pos[0]));
    for (uint i = 0; i < 3; i++) {
        frag_world_pos = geom_world_pos[i];
        //frag_world_pos = gl_in[i].gl_Position.xyz;
        //gl_Position = gl_in[i].gl_Position;
        frag_normal = geom_normal[i];
        frag_albedo = geom_albedo[i];
        frag_axis = 2.0;
        if (face_normal.x > face_normal.y && face_normal.x > face_normal.z) {
            // Look from +X
            frag_world_pos = frag_world_pos.zyx;
            frag_axis = 0.0;
        }
        else if (face_normal.y > face_normal.x && face_normal.y > face_normal.z) {
            // Look from +Y
            frag_world_pos = frag_world_pos.xzy;
            frag_axis = 1.0;
        }

        // Orthogonal projection to unit cube (NDC)
        gl_Position = projection * world_to_view * vec4(frag_world_pos, 1.0);
        //frag_world_pos = (projection * vec4(frag_world_pos, 1.0)).xyz;
        EmitVertex();
    }
    EndPrimitive();
}