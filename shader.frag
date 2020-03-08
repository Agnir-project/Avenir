#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(early_fragment_tests) in;

layout(location = 0) in vec4 in_pos;
layout(location = 1) in vec3 frag_norm;
layout(location = 2) in vec4 frag_color;
layout(location = 0) out vec4 color;

void main() {
    color = frag_color;
}
