#version 450

in vec2 tex_coord;
out vec4 color;

uniform sampler2D cascade;
uniform sampler2D scene_normal;
uniform sampler2D scene_albedo;
uniform sampler2D scene_emissive;
uniform float cascade_index;

uniform float ambient;

layout(std430) readonly buffer RCConstants {
    vec2 c0_resolution;
    float num_cascades;
    float c0_probe_spacing;
    float c0_interval_length;
    float normal_offset;
    float gamma;
    float ambient_occlusion_factor;
    float diffuse_intensity;
    float ambient_occlusion;
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

vec3 linear_to_srgb(vec3 c) {
    return pow(c.rgb, vec3(1.0 / gamma));
}

vec3 srgb_to_linear(vec3 c) {
    return pow(c.rgb, vec3(gamma));
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
    vec3 normal = octahedral_decode(texture(scene_normal, tex_coord).xy);

    float altitudinal_dirs = 4.0 * pow(2.0, cascade_index);
    float altitudinal_dirs_inv = 1.0 / altitudinal_dirs;
    float azimuthal_dirs = 4.0 * pow(2.0, cascade_index);
    float azimuthal_dirs_inv = 1.0 / azimuthal_dirs;
    vec2 scale_bias = vec2(azimuthal_dirs_inv, altitudinal_dirs_inv);

    vec4 radiance = vec4(0.0);
    float total_cone_weight = 0.0;
    for (float alt = 0.0; alt < altitudinal_dirs; alt += 1.0) {
        const float altitude = (alt + 0.5) * (3.14159265 * altitudinal_dirs_inv);
        const float cos_altitude = cos(altitude); 
        const float sin_altitude = sin(altitude); 

        for (float azi = 0.0; azi < azimuthal_dirs; azi++) {
            const vec2 cone_coord = vec2(tex_coord * scale_bias + vec2(azi, alt) * scale_bias);
            const vec4 cone_radiance = texture(cascade, cone_coord);

            const float azimuth = (azi + 0.5) * (2.0 * 3.14159265 * azimuthal_dirs_inv);
            const vec3 cone_direction = normalize(vec3(
                cos(azimuth) * sin_altitude,
                cos_altitude,
                sin(azimuth) * sin_altitude
            ));

            float cone_weight = max(0.0, dot(cone_direction, normal));
            radiance += cone_radiance * cone_weight;
            total_cone_weight += cone_weight;
        }
    }

    radiance = (total_cone_weight > 0.0) ? radiance / total_cone_weight : vec4(0.0, 0.0, 0.0, 1.0);
    radiance /= altitudinal_dirs * azimuthal_dirs;
    radiance.a *= ambient_occlusion_factor;

    const vec4 albedo = texture(scene_albedo, tex_coord);
    const vec3 emissive = texture(scene_emissive, tex_coord).rgb;
    vec3 diffuse = srgb_to_linear(albedo.rgb) * radiance.rgb * diffuse_intensity;
    diffuse = clamp(diffuse, 0.0, 1.0);

    vec3 direct = ambient + emissive;
    direct = clamp(direct, 0.0, 1.0);
    direct = (ambient_occlusion != 0.0) ? direct * radiance.a : direct;

    vec3 out_color = diffuse + direct;
    out_color = clamp(out_color, 0.0, 1.0);
    color = vec4(linear_to_srgb(out_color), 1.0);
}