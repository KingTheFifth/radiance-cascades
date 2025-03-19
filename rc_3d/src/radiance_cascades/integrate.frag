#version 450

in vec2 tex_coord;
out vec4 color;

uniform sampler2D merged_cascade_0;
uniform sampler2D scene_normal;
uniform sampler2D scene_albedo;
uniform sampler2D scene_emissive;

layout(std430) readonly buffer RCConstants {
    vec2 c0_resolution;
    float num_cascades;
    float c0_probe_spacing;
    float c0_interval_length;
};

layout(std430) readonly buffer SceneMatrices {
    mat4 world_to_view;
    mat4 world_to_view_inv;
    mat4 perspective;
    mat4 perspective_inv;
    vec2 screen_res;
    vec2 screen_res_inv;
};

const float altitudes[4] = {acos(-0.75), acos(-0.25), acos(0.25), acos(0.75)};

vec3 srgb_to_linear(vec3 c) {
    return pow(c.rgb, vec3(1.0 / 1.6));
}

vec3 octahedral_decode(vec2 v) {
    // Based on https://knarkowicz.wordpress.com/2014/04/16/octahedron-normal-vector-encoding/
    //vec2 v_adjusted = 2.0 * v - 1.0;
    vec2 v_adjusted = v;
    vec3 n = vec3(v_adjusted.xy, 1.0 - abs(v_adjusted.x) - abs(v_adjusted.y));
    float t = max((-n.z), 0.0);
    return normalize(vec3(
        n.x + ((n.x >= 0.0) ? (-t) : t),
        n.y + ((n.y >= 0.0) ? (-t) : t),
        n.z
    ));
}

void main() {
    vec3 radiance = vec3(0.0);
    vec3 normal = octahedral_decode(texture(scene_normal, tex_coord).xy);
    for (float alt = 0.0; alt < 4.0; alt += 1.0) {
        const float cos_altitude = cos(altitudes[int(alt)]); 
        const float sin_altitude = sin(altitudes[int(alt)]); 

        for (float azi = 0.0; azi < 4.0; azi++) {
            const vec2 cone_coord = vec2(tex_coord * 0.25 + vec2(azi, alt) * 0.25);
            const vec3 cone_radiance = texture(merged_cascade_0, cone_coord).rgb;

            const float azimuth = (azi + 0.5) * (2.0 * 3.14169265 * 0.25);
            const vec3 cone_direction = normalize(mat3(world_to_view) * vec3(
                cos(azimuth) * sin_altitude,
                cos_altitude,
                sin(azimuth) * sin_altitude
            ));

            radiance += cone_radiance * dot(cone_direction, normal);
        }
    }

    const vec4 albedo = texture(scene_albedo, tex_coord);
    const vec3 emissive = texture(scene_emissive, tex_coord).rgb;
    color = vec4(srgb_to_linear(albedo.rgb * radiance + emissive), albedo.a);
    //color = vec4(srgb_to_linear(texture(merged_cascade_0, tex_coord).rgb), 1.0);
}