#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (binding = 0) uniform UBO{
    mat4 model_to_screen;
    vec4 color;
} ubo;

layout (location = 0) in vec4 pos;
layout (location = 1) in vec2 uv;

layout (location = 0) out vec2 o_uv;
layout (location = 1) out vec3 o_pos;

void main() {
    o_uv = uv;
    o_pos = pos.xyz;
    gl_Position = ubo.model_to_screen * pos;
}
