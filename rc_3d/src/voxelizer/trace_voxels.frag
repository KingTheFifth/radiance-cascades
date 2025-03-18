#version 450

in vec2 tex_coord;
out vec4 color;

//layout(binding = 0, rgba16f) uniform readonly image3D voxel_tex;
uniform sampler3D voxel_tex;

uniform vec3 cam_pos;
uniform vec3 pixel_down_left;
uniform vec3 pixel_delta_u;
uniform vec3 pixel_delta_v;
uniform mat4 world_to_voxel;
uniform float step_length;
uniform float step_count;

void main() {
    const vec3 fragment_world_pos = pixel_down_left + pixel_delta_u * gl_FragCoord.x + pixel_delta_v * gl_FragCoord.y;
    const vec3 ray_origin = cam_pos; 
    const vec3 ray_direction = normalize(fragment_world_pos - ray_origin);

    color = vec4(0.0);
    for (float s = 0.0; s < step_count && color.a < 0.99; s++) {
        const vec3 curr_point = ray_origin + ray_direction * s * step_length;
        const vec3 sample_point = (world_to_voxel * vec4(curr_point, 1.0)).xyz;
        //const vec4 curr_sample = imageLoad(voxel_tex, ivec3(sample_point));
        const vec4 curr_sample = texture(voxel_tex, sample_point);
        color += (1.0 - color.a) * curr_sample;
    }
}