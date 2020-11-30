#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (binding = 0) uniform UBO {
    uint depth_pyramid_dimension;	// pow2 y dimension of mip 0 (texture x is 1.5x wider)
} ubo;

layout (binding = 1, r32ui) uniform uimage2D debug_tex;

uvec4 calculate_mip_rect(uint dimensions, uint mip)
{
    uint pixels_mip = dimensions >> mip;
    uvec4 uv_rect = uvec4(0, 0, pixels_mip, pixels_mip);
    if (mip > 0)
    {
        uv_rect.x = dimensions;
        uv_rect.y = dimensions - pixels_mip * 2;
    }
    return uv_rect;
}

vec4 calculate_mip_rect_uv(uint dimensions, uint mip)
{
    float inv_mip_exp2 = exp2(-mip);
    float x_scale = (2.0 / 3.0);
    vec4 uv_rect = vec4(0.0, 0.0, inv_mip_exp2 * x_scale, inv_mip_exp2);
    if (mip > 0)
    {
        uv_rect.x = x_scale;
        uv_rect.y = 1.0 - inv_mip_exp2 * 2.0;
    }
    return uv_rect;
}

layout (location = 0) in vec2 o_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    vec3 color = vec3(0, 0, 0);
    for (uint mip = 0; mip < 7; ++mip)
    {
        uvec4 mip_rect = calculate_mip_rect(ubo.depth_pyramid_dimension, mip);
        ivec2 uv = ivec2(mip_rect.xy) + ivec2(o_uv * vec2(mip_rect.zw));
        uint i = imageLoad(debug_tex, uv).x;

        float v = float(i) * 0.25;
        uint channel = mip % 3;
        if (channel == 0) color.r += v;
        else if (channel == 1) color.g += v;
        else if (channel == 2) color.b += v;
    }
    uFragColor = vec4(color, 1.0);
}
