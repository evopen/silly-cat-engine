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
    vec3 world_normal;
    vec3 world_position;
};

// Gets hit info about the object at the intersection. This uses GLSL variables
// defined in closest hit stages instead of ray queries.
HitInfo get_object_hit_info()
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
    vec3 objectPosition = v0 * barycentrics.x + v1 * barycentrics.y + v2 * barycentrics.z;
    // Transform from object space to world space:
    result.world_position = gl_ObjectToWorldEXT * vec4(objectPosition, 1.0f);

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
    result.world_normal = normalize((objectNormal * gl_WorldToObjectEXT).xyz);

    // Flip the normal so it points against the ray direction:
    const vec3 rayDirection = gl_WorldRayDirectionEXT;
    result.world_normal = faceforward(result.world_normal, rayDirection, result.world_normal);

    float dotx = dot(result.world_normal, vec3(1, 0, 0));
    if (dotx > 0.99) {
        result.color = vec3(0.8, 0.2, 0.2);
    } else if (dotx < -0.99) {
        result.color = vec3(0.2, 0.8, 0.2);
    } else {
        result.color = vec3(0.7);
    }

    return result;
}

void main()
{

    HitInfo hit_info = get_object_hit_info();

    payload.color = hit_info.color;
    payload.rayHitSky = false;
    payload.rayOrigin = hit_info.world_position;
    // For a random diffuse bounce direction, we follow the approach of
    // Ray Tracing in One Weekend, and generate a random point on a sphere
    // of radius 1 centered at the normal. This uses the random_unit_vector
    // function from chapter 8.5:
    const float theta = 6.2831853 * stepAndOutputRNGFloat(payload.rngState); // Random in [0, 2pi]
    const float u = 2.0 * stepAndOutputRNGFloat(payload.rngState) - 1.0; // Random in [-1, 1]
    const float r = sqrt(1.0 - u * u);

    payload.rayDirection = normalize(hit_info.world_normal + vec3(r * cos(theta), r * sin(theta), u));
}