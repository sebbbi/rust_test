#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (binding = 0) uniform UBO {
    mat4 world_to_screen;
    vec4 color;
    vec4 center_to_edge;
} ubo;

struct InstanceData
{
	vec4 position;
};

layout(std430, binding = 1) buffer Instances
{
    InstanceData instances[];
};

layout (location = 0) in vec3 o_uvw;
layout (location = 0) out vec4 uFragColor;

uint hash1(uint n) 
{
    // integer hash copied from Hugo Elias
	n = (n << 13U) ^ n;
    n = n * (n * n * 15731U + 789221U) + 1376312589U;
    return n;
}

void main() {
    uint prim_id = gl_PrimitiveID;
    uint hash = hash1(prim_id);

    vec3 color = vec3(float(hash & 0xff) / 255.0f, float((hash>>8) & 0xff) / 255.0f, float((hash>>16) & 0xff) / 255.0f);

    uFragColor = vec4(color, 1.0);

    //uFragColor = vec4(o_uvw, 1.0);
}
