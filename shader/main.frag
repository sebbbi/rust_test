#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (binding = 1) uniform sampler3D samplerColor;

layout (binding = 0) uniform UBO {
    mat4 model_to_world;
    mat4 world_to_model;
    mat4 model_to_screen;
    vec4 color;
    vec4 camera_position;
    vec4 volume_scale;
    vec4 center_to_edge;
} ubo;

layout (location = 0) in vec3 o_uvw;
layout (location = 1) in vec3 o_local_camera_pos;
layout (location = 2) in vec3 o_local_pos;

layout (location = 0) out vec4 uFragColor;

bool outside(vec3 uwv) {
    // saturate instructions are free
    if (uwv.x != clamp(uwv.x, 0.0, 1.0)) return true;
    if (uwv.y != clamp(uwv.y, 0.0, 1.0)) return true;
    if (uwv.z != clamp(uwv.z, 0.0, 1.0)) return true;
    return false;
}

void main() {
    vec3 ray_pos = o_uvw;
    vec3 ray_dir = normalize(o_local_pos - o_local_camera_pos);
    ray_dir *= ubo.volume_scale.xyz;
    float d = 0;
    bool discarded = false;
    while (true) {
        vec3 uvw = ray_pos + ray_dir * d;
        if (outside(uvw)) {
            discarded = true;
            //discard;
            break;
        }
        float s = texture(samplerColor, uvw).x;
        s = s * 2.0 - 1.0;
        d += s;
        if (s < 0.001) break;
    }

    if (discarded) {
        uFragColor = vec4(1.0, 0.0, 0.0, 1.0);
    }
    else {
        uFragColor = vec4(0.0, 0.0, 1.0, 1.0) + ubo.color * d;
    }
}
