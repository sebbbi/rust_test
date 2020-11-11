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

layout (location = 0) in vec3 o_uvw;
layout (location = 1) in vec4 o_local_camera_pos_lod;
layout (location = 2) in vec3 o_local_pos;

layout (location = 0) out vec4 uFragColor;

bool outside(vec3 uwv) {
    // saturate instructions are free
    if (uwv.x != clamp(uwv.x, 0.0, 1.0)) return true;
    if (uwv.y != clamp(uwv.y, 0.0, 1.0)) return true;
    if (uwv.z != clamp(uwv.z, 0.0, 1.0)) return true;
    return false;
}

vec3 normal(vec3 uvw) {

    float lod = o_local_camera_pos_lod.w;
    vec3 e = ubo.texel_scale.xyz * 0.5;
    float xm = textureLod(samplerColor, uvw + vec3(-e.x, 0,    0), lod).x;
    float xp = textureLod(samplerColor, uvw + vec3( e.x, 0,    0), lod).x;
    float ym = textureLod(samplerColor, uvw + vec3( 0,   -e.y, 0), lod).x;
    float yp = textureLod(samplerColor, uvw + vec3( 0,   e.y,  0), lod).x;
    float zm = textureLod(samplerColor, uvw + vec3( 0,   0, -e.z), lod).x;
    float zp = textureLod(samplerColor, uvw + vec3( 0,   0,  e.z), lod).x;
    return normalize(vec3(xp - xm, yp - ym, zp - zm));
}

void main() {
    vec3 ray_pos = o_uvw;
    vec3 ray_dir = normalize(o_local_pos - o_local_camera_pos_lod.xyz);

    ray_dir *= ubo.volume_scale.xyz;
    float d = 0;
    for (uint i=0; i<1024; ++i) {
        vec3 uvw = ray_pos + ray_dir * d;
        if (outside(uvw)) {
            discard;
            break;
        }
        float s = textureLod(samplerColor, uvw, o_local_camera_pos_lod.w).x;
        s = s * 2.0 - 1.0;
        d += s;
        if (s < 0.00025) break;
    }
    uFragColor = vec4(normal(ray_pos + ray_dir * d), 1.0);
}
