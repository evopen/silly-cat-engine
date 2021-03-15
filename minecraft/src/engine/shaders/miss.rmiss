#version 460 core
#extension GL_EXT_ray_tracing : require
#extension GL_GOOGLE_include_directive : require

#include "common.glsl"

layout(location = 0) rayPayloadInEXT PassableInfo payload;

void main()
{
    payload.rayHitSky = true;

    const float ray_direction_y = gl_WorldRayDirectionEXT.y;
    if (ray_direction_y > 0) {
        payload.color = mix(vec3(1.0), vec3(0.25, 0.5, 1), ray_direction_y);
    } else {
        payload.color = vec3(0.03);
    }
}
