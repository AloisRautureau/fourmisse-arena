#version 450

layout(location = 0) in vec3 in_frag_pos;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec3 in_colour;

layout(set = 1, binding = 1) uniform Material {
    float shininess;
    float spec_intensity;
} material;

layout(location = 0) out vec3 out_colour;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec3 out_frag_pos;
layout(location = 3) out vec2 out_specular;

void main() {
    out_colour = in_colour;
    out_normal = in_normal;
    out_frag_pos = in_frag_pos;
    out_specular = vec2(material.spec_intensity, material.shininess);
}