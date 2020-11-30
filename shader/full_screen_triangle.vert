#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) out vec2 o_uv;

void main() {
    uint vx = gl_VertexIndex;
    uvec2 uv = uvec2(vx & 0x1, vx >> 1);
    vec2 pos = uv * 4.0 - 1.0;

    o_uv = uv * 2.0;
    gl_Position = vec4(pos, 1.0, 1.0);
}
