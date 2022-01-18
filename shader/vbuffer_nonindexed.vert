#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// TODO: Move to UBO
#define GRID_DIM 7
//#define NUM_GRID_VERTICES (GRID_DIM_VX * GRID_DIM_VX)
#define NUM_GRID_INDICES (GRID_DIM * GRID_DIM * 3 * 2)

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
layout (location = 1) flat out uint o_prim_index;

void main() {
    uint vx = gl_VertexIndex;

    uint instance = vx / NUM_GRID_INDICES;
    uint instance_local_index = vx - instance * NUM_GRID_INDICES;
    uint instance_local = instance_local_index / (3 * 2);

    uint cell_index = instance_local_index - instance_local * (3 * 2);

    uvec2 offset;
    if (cell_index < 3)
    {
        // Upper left triangle
        offset.x = cell_index % 2;
        offset.y = cell_index / 2;
    }
    else
    {
        // Lower right triangle
        offset.x = (cell_index - 2) / 2;
        offset.y = (cell_index - 2) % 2;
    }

    uint x = instance_local % GRID_DIM + offset.x;
    uint y = instance_local / GRID_DIM + offset.y;

    uvec3 xyz = uvec3(x, y, 0);
    vec3 uvw = vec3(xyz) * (1.0 / GRID_DIM);
    vec3 pos = uvw * 2.0 - 1.0;

    vec3 instance_pos = instances[instance].position.xyz;

    vec3 local_pos = pos.xyz * ubo.center_to_edge.xyz;

    o_uvw = uvw;
    o_prim_index = vx / 3;
    gl_Position = ubo.world_to_screen * vec4(local_pos + instance_pos, 1.0);
}
