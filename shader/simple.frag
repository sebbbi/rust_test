#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (binding = 0) uniform UBO {
    mat4 world_to_screen;
    vec4 color;
    vec4 camera_position;
    vec4 volume_scale;
    vec4 center_to_edge;
    vec4 texel_scale;
} ubo;

struct InstanceData
{
	vec4 position;
};

struct VisibilityData
{
	uint index;
};

layout(std430, binding = 1) buffer Instances
{
    InstanceData instances[];
};

layout(std430, binding = 2) buffer Visibility
{
    VisibilityData visibility[];
};

layout (binding = 3) uniform sampler3D samplerSDF;

layout (location = 0) in vec3 o_uvw;
layout (location = 1) in vec4 o_local_camera_pos_lod;
layout (location = 2) in vec3 o_local_pos;

layout (location = 0) out vec4 uFragColor;

void main() {
    uFragColor = vec4(o_uvw, 1.0);
}
