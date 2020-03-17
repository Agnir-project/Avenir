#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(early_fragment_tests) in;

layout(location = 0) in vec4 in_pos;
layout(location = 1) in vec3 frag_norm;
layout(location = 2) in vec4 frag_color;
layout(location = 0) out vec4 color;

layout(set = 0, binding = 0) uniform Args {
    mat4 proj;
    mat4 view;
    float ambient_power;
};

void main() {
    color = frag_color * vec4(frag_norm * ambient_power, 1.0);
}
