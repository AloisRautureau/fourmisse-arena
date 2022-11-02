#version 450

layout(location = 0) in vec3 position;

layout(set = 0, binding = 2) uniform MVP {
    mat4 model;
    mat4 view;
    mat4 projection;
} mvp;

layout(location = 0) out vec3 out_position;

void main() {
    mat4 worldview = mvp.view * mvp.model;
    gl_Position = mvp.projection * worldview * vec4(position, 1.0);
    out_position = vec3(mvp.model * vec4(position, 1.0));
}