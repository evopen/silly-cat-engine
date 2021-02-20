#version 460 core
#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_shader_16bit_storage : require
#extension GL_EXT_debug_printf : require
#extension GL_GOOGLE_include_directive : require

#include "common.glsl"

layout(location = 0) rayPayloadInEXT PassableInfo payload;

hitAttributeEXT vec2 attributes;

layout(binding = 2, set = 0, scalar) buffer Indices
{
    uint16_t indices[];
};
layout(binding = 3, set = 0, scalar) buffer Vertices
{
    vec3 vertices[];
};

struct HitInfo {
    vec3 color;
    vec3 objectPosition;
    vec3 worldPosition;
    vec3 worldNormal;
};

// Gets hit info about the object at the intersection. This uses GLSL variables
// defined in closest hit stages instead of ray queries.
HitInfo getObjectHitInfo()
{
    HitInfo result;
    // Get the ID of the triangle
    const int primitiveID = gl_PrimitiveID;

    // Get the indices of the vertices of the triangle
    const uint i0 = uint(indices[3 * primitiveID + 0]);
    const uint i1 = uint(indices[3 * primitiveID + 1]);
    const uint i2 = uint(indices[3 * primitiveID + 2]);

    // Get the vertices of the triangle
    const vec3 v0 = vertices[i0];
    const vec3 v1 = vertices[i1];
    const vec3 v2 = vertices[i2];

    // Get the barycentric coordinates of the intersection
    vec3 barycentrics = vec3(0.0, attributes.x, attributes.y);
    barycentrics.x = 1.0 - barycentrics.y - barycentrics.z;

    // Compute the coordinates of the intersection
    result.objectPosition = v0 * barycentrics.x + v1 * barycentrics.y + v2 * barycentrics.z;
    // Transform from object space to world space:
    result.worldPosition = gl_ObjectToWorldEXT * vec4(result.objectPosition, 1.0f);

    // Compute the normal of the triangle in object space, using the right-hand rule:
    //    v2      .
    //    |\      .
    //    | \     .
    //    |/ \    .
    //    /   \   .
    //   /|    \  .
    //  L v0---v1 .
    // n
    const vec3 objectNormal = cross(v1 - v0, v2 - v0);
    // Transform normals from object space to world space. These use the transpose of the inverse matrix,
    // because they're directions of normals, not positions:
    result.worldNormal = normalize((objectNormal * gl_WorldToObjectEXT).xyz);

    // Flip the normal so it points against the ray direction:
    const vec3 rayDirection = gl_WorldRayDirectionEXT;
    result.worldNormal = faceforward(result.worldNormal, rayDirection, result.worldNormal);
    result.color = vec3(0.7);

    return result;
}

void main()
{

    HitInfo hit_info = getObjectHitInfo();

    payload.color = hit_info.color;
    payload.rayHitSky = false;
    payload.rayOrigin = hit_info.worldPosition;
    payload.rayDirection = reflect(gl_WorldRayDirectionEXT, hit_info.worldNormal);
}