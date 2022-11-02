#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 color;

layout(location = 0) out vec3 out_color;
layout(location = 1) out vec3 out_normal;

layout(set = 0, binding = 0) uniform MVP {
    mat4 model;
    mat4 view;
    mat4 projection;
} mvp;

void main() {
    mat4 worldview = mvp.view * mvp.model;
    gl_Position = mvp.projection * worldview * vec4(position, 1.0);

    out_color = color;
    out_normal = mat3(mvp.model) * normal;
}