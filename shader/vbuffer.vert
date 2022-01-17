#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#define USE_VISIBILITY_DATA

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

layout (location = 0) out vec3 o_uvw;

void main() {
    uint vx = gl_VertexIndex;
    uint instance = vx >> 3;

    uvec3 xyz = uvec3(vx & 0x1, (vx & 0x4) >> 2, (vx & 0x2) >> 1);
    vec3 uvw = vec3(xyz);
    vec3 pos = uvw * 2.0 - 1.0;

    vec3 instance_pos = instances[instance].position.xyz;

    vec3 local_pos = pos.xyz * ubo.center_to_edge.xyz;

    o_uvw = uvw;
    gl_Position = ubo.world_to_screen * vec4(local_pos + instance_pos, 1.0);
}
