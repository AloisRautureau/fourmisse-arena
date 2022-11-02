#version 450

layout(location = 0) in vec3 in_position;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_vertex_color;

layout(set = 0, binding = 2) uniform AmbientLight {
    vec3 color;
    float intensity;
} ambient;

layout(location = 0) out vec4 out_color;

void main() {
    vec3 ambient_color = ambient.intensity * ambient.color;
    out_color = vec4(ambient_color * subpassLoad(u_vertex_color).rgb, 1.0);
}