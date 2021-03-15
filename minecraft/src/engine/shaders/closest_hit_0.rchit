#version 460 core
#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_shader_16bit_storage : require
#extension GL_EXT_debug_printf : require
#extension GL_GOOGLE_include_directive : require

#include "closest_hit_common.glsl"

// diffuse reflection off a 70% reflective surface
void main()
{
    HitInfo hit_info = get_object_hit_info();

    payload.color = vec3(0.7);
    payload.rayHitSky = false;
    payload.rayOrigin = hit_info.world_position;

    payload.rayDirection = diffuseReflection(hit_info.world_normal, payload.rngState);
}