#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 colour;

layout(set = 0, binding = 0) uniform VP {
    mat4 view;
    mat4 projection;
} vp;
layout(set = 1, binding = 0) uniform Model {
    mat4 model_transform;
    mat4 normals_transform;
} model;

layout(location = 0) out vec3 out_frag_pos;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec3 out_colour;

void main() {
    mat4 vp_matrix = vp.projection * vp.view;
    vec4 frag_position = model.model_transform * vec4(position, 1.0);
    gl_Position = vp_matrix * frag_position;

    out_frag_pos = frag_position.xyz;
    out_normal = mat3(model.normals_transform) * normal;
    out_colour = colour;
}