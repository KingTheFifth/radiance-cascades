#version 450

layout(triangles) in;
layout(triangle_strip, max_vertices = 3) out;

in vec3 geom_world_pos[];
in vec3 geom_normal[];
in vec4 geom_albedo[];

out vec3 frag_world_pos;
out vec3 frag_normal;
out vec4 frag_albedo;

void main() {
    const vec3 face_normal = abs(cross(geom_world_pos[1] - geom_world_pos[0], geom_world_pos[2] - geom_world_pos[0]));
    for (uint i = 0; i < 3; i++) {
        frag_world_pos = geom_world_pos[i];
        frag_normal = geom_normal[i];
        frag_albedo = geom_albedo[i];
        if (face_normal.x > face_normal.y && face_normal.x > face_normal.z) {
            gl_Position = vec4(frag_world_pos.y, frag_world_pos.z, 0.0, 1.0);
        }
        else if (face_normal.y > face_normal.x && face_normal.y > face_normal.z) {
            gl_Position = vec4(frag_world_pos.x, frag_world_pos.z, 0.0, 1.0);
        }
        else {
            gl_Position = vec4(frag_world_pos.x, frag_world_pos.y, 0.0, 1.0);
        }
        EmitVertex();
    }
    EndPrimitive();
}