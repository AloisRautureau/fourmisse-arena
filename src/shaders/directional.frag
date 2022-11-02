#version 450

layout(location = 0) in vec3 in_position;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_vertex_color;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normal;

layout(set = 0, binding = 3) uniform DirectionalLight {
    vec3 color;
    float intensity;
    vec3 position;
} directional;

layout(location = 0) out vec4 out_color;

void main() {
    vec3 light_dir = normalize(directional.position.xyz - in_position);
    float directional_intensity = max(dot(normalize(subpassLoad(u_normal).rgb), light_dir), 0.0) * directional.intensity;
    vec3 directional_color = directional_intensity * directional.color;

    out_color = vec4(directional_color * subpassLoad(u_vertex_color).rgb, 1.0);
}