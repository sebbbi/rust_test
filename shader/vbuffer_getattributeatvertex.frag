#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_NV_fragment_shader_barycentric : enable

// TODO: Move to UBO
#define GRID_DIM_VX 8
#define NUM_GRID_VERTICES (GRID_DIM_VX * GRID_DIM_VX)

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
layout (location = 1) pervertexNV in uint o_vert_id[3];
layout (location = 0) out vec4 uFragColor;

uint hash1(uint n) 
{
    // integer hash copied from Hugo Elias
	n = (n << 13U) ^ n;
    n = n * (n * n * 15731U + 789221U) + 1376312589U;
    return n;
}

void swapminmax(inout uint a, inout uint b)
{
    uint v0 = min(a, b);
    b = max(a, b);
    a = v0;
}

void main() {
    uint v0 = o_vert_id[0];
    uint v1 = o_vert_id[1];
    uint v2 = o_vert_id[2];

    // Optimal sorting network for 3 values
    swapminmax(v1, v2);
    swapminmax(v0, v1);
    swapminmax(v1, v2);

    // Smallest vertex defines the grid index. If v0 and v1 are adjacent (same scanline), we are processing the upper left triangle
    uint lower = 1;
    uint grid_index = v0 - 1;
    if (v0 + 1 == v1) { grid_index = grid_index + 1; lower = 0; }

    uint instance_id = grid_index / (GRID_DIM_VX * GRID_DIM_VX);
    uint instance_local = grid_index - instance_id * (GRID_DIM_VX * GRID_DIM_VX);

    uint grid_y = instance_local / GRID_DIM_VX;
    uint grid_x = instance_local - grid_y * GRID_DIM_VX;

    uint prim_id = instance_id * (GRID_DIM_VX - 1) * (GRID_DIM_VX - 1) * 2 + grid_y * (GRID_DIM_VX - 1) * 2 + grid_x * 2 + lower;

    uint hash = hash1(prim_id);

    vec3 hashColor = vec3(float(hash & 0xff) / 255.0f, float((hash>>8) & 0xff) / 255.0f, float((hash>>16) & 0xff) / 255.0f);

    uFragColor = vec4(hashColor * 0.9 + o_uvw * 0.1, 1.0);
}
