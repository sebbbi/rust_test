#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (binding = 0) uniform UBO {
    mat4 model_to_world;
    mat4 world_to_model;
    mat4 model_to_screen;
    vec4 color;
    vec4 camera_position;
    vec4 volume_scale;
    vec4 center_to_edge;
} ubo;

layout (location = 0) in vec4 pos;
layout (location = 1) in vec2 uv;

layout (location = 0) out vec3 o_uvw;
layout (location = 1) out vec3 o_local_camera_pos;
layout (location = 2) out vec3 o_local_pos;

void main() {
    vec3 local_pos = pos.xyz * ubo.center_to_edge.xyz;
    vec3 local_camera_pos = (ubo.world_to_model * vec4(ubo.camera_position.xyz, 1.0)).xyz;

    o_uvw = pos.xyz * 0.5 + 0.5;
    o_local_pos = local_pos;
    o_local_camera_pos = local_camera_pos;
    gl_Position = ubo.model_to_screen * vec4(local_pos, 1.0);
}
