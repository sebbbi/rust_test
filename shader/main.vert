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

layout(std140, binding = 1) buffer Instances
{
    InstanceData instances[];
};

layout (binding = 2) uniform sampler3D samplerColor;

layout (location = 0) out vec3 o_uvw;
layout (location = 1) out vec4 o_local_camera_pos_lod;
layout (location = 2) out vec3 o_local_pos;

void main() {
    uint vx = gl_VertexIndex;
    uint instance = vx >> 3;
    uvec3 xyz = uvec3(vx & 0x1, (vx & 0x4) >> 2, (vx & 0x2) >> 1);
    vec3 uvw = vec3(xyz);
    vec3 pos = uvw * 2.0 - 1.0;

    vec3 instance_pos = instances[instance].position.xyz;

    vec3 local_pos = pos.xyz * ubo.center_to_edge.xyz;
    vec3 local_camera_pos = ubo.camera_position.xyz - instance_pos;

    float lod = 0.5 * log2(dot(local_camera_pos, local_camera_pos)) - 6.0;

    vec3 texel_scale_lod = ubo.texel_scale.xyz * exp2(max(lod, 0.0));

    o_uvw = uvw * (vec3(1.0) - texel_scale_lod) + texel_scale_lod * 0.5;
    o_local_pos = local_pos;
    o_local_camera_pos_lod = vec4(local_camera_pos, lod);
    gl_Position = ubo.world_to_screen * vec4(local_pos + instance_pos, 1.0);
}
