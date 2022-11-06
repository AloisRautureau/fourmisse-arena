#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_vertex_color;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput u_normal;
layout(input_attachment_index = 1, set = 0, binding = 2) uniform subpassInput u_frag_pos;
layout(input_attachment_index = 1, set = 0, binding = 3) uniform subpassInput u_specular;

layout(set = 0, binding = 4) uniform Camera {
    vec3 position;
} camera;
layout(set = 0, binding = 5) uniform DirectionalLight {
    vec3 color;
    float intensity;
    vec3 position;
} directional;

layout(location = 0) out vec4 out_color;

void main() {
    vec3 normal = subpassLoad(u_normal).xyz;
    float spec_shininess = subpassLoad(u_specular).y;

    vec3 view_dir = -normalize(camera.position - subpassLoad(u_frag_pos).xyz);
    vec3 light_dir = normalize(directional.position.xyz - subpassLoad(u_normal).xyz);
    vec3 reflect_dir = reflect(-light_dir, normal);

    float spec_intensity = pow(max(dot(view_dir, reflect_dir), 0.0), spec_shininess);
    vec3 spec_color = spec_intensity * directional.color;

    float directional_intensity = max(dot(normalize(subpassLoad(u_normal).rgb), light_dir), 0.0) * directional.intensity;
    vec3 directional_color = directional_intensity * directional.color;

    vec3 final_color = (spec_color + directional_color) * subpassLoad(u_vertex_color).rgb;
    out_color = vec4(final_color, 1.0);
}
