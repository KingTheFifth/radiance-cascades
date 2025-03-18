#version 450

layout(triangles) in;
layout(triangle_strip, max_vertices = 3) out;

in vec3 geom_world_pos[];
in vec3 geom_normal[];
in vec4 geom_albedo[];

out vec4 frag_world_pos;
out vec3 frag_normal;
out vec4 frag_albedo;
out int frag_axis;

uniform mat4 projection_x;
uniform mat4 projection_y;
uniform mat4 projection_z;

void main() {
    const vec3 face_normal = abs(normalize(cross(geom_world_pos[1] - geom_world_pos[0], geom_world_pos[2] - geom_world_pos[0])));
    for (uint i = 0; i < 3; i++) {
        frag_world_pos = vec4(geom_world_pos[i], 1.0);
        //frag_world_pos = gl_in[i].gl_Position.xyz;
        //gl_Position = gl_in[i].gl_Position;
        frag_normal = geom_normal[i];
        frag_albedo = geom_albedo[i];

        mat4 projection = projection_z;
        frag_axis = 2;

        if (face_normal.x > face_normal.y && face_normal.x > face_normal.z) {
            // Look from +X
            projection = projection_x;
            frag_axis = 0;
        }
        else if (face_normal.y > face_normal.x && face_normal.y > face_normal.z) {
            // Look from +Y
            projection = projection_y;
            frag_axis = 1;
        }

        // Orthogonal projection to unit cube (NDC)
        gl_Position = projection * frag_world_pos;
        EmitVertex();
    }
    EndPrimitive();
}