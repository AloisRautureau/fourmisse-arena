#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_vertex_color;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normal;
layout(input_attachment_index = 1, set = 0, binding = 2) uniform subpassInput u_frag_pos;
layout(input_attachment_index = 1, set = 0, binding = 3) uniform subpassInput u_specular;

layout(set = 0, binding = 4) uniform Camera {
    vec3 position;
} camera;
layout(set = 0, binding = 5) uniform LightSource {
    vec4 vector;
    vec3 color;
} light;

layout(location = 0) out vec4 out_color;

// Cell shading
const float cell_color_levels = 16;
const float cell_scale_factor = 1 / cell_color_levels;

// Attenuation constants
const float Kc = 1;
const float Kl = 0.7;
const float Kq = 1.8;

void main() {
    vec3 vertex_color = subpassLoad(u_vertex_color).rgb;
    vec3 normal = subpassLoad(u_normal).xyz;
    vec3 frag_pos = subpassLoad(u_frag_pos).xyz;
    float spec_intensity = subpassLoad(u_specular).x;
    float spec_shininess = subpassLoad(u_specular).y;

    vec3 view_dir = normalize(camera.position - frag_pos);
    vec3 light_dir;
    float attenuation_factor;
    if(light.vector.w == 0.0) {
        // We're rendering a directional light source
        light_dir = normalize(-light.vector.xyz);
        float light_dist = length(light.vector.xyz - frag_pos);
        attenuation_factor = 1;
    } else if (light.vector.w == 1.0) {
        // We're rendering a point light
        light_dir = normalize(light.vector.xyz - frag_pos);
        float light_dist = length(light.vector.xyz - frag_pos);
        attenuation_factor = 1 / (Kc + Kl * light_dist + Kq * (light_dist * light_dist));
    }
    vec3 reflect_dir = reflect(light_dir, normal);

    float specular_intensity = pow(max(dot(view_dir, reflect_dir), 0), spec_shininess) * spec_intensity * attenuation_factor;
    vec3 specular_color = specular_intensity * light.color;

    float diffuse_intensity = max(dot(normal, light_dir), 0) * attenuation_factor;
    float diffuse_factor = ceil(diffuse_intensity * cell_color_levels) * cell_scale_factor;
    vec3 diffuse_color = diffuse_intensity * light.color;

    vec3 final_color = (diffuse_color + specular_color) * vertex_color;
    out_color = vec4(final_color, 1);
}
