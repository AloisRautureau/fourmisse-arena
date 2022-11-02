#version 450
layout(location = 0) in vec3 in_color;
layout(location = 1) in vec3 in_normal;

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec3 out_normal;

void main() {
    out_color = vec4(in_color, 1.0);
    out_normal = in_normal;
}