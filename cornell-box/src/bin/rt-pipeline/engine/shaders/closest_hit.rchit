#version 460 core
#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_shader_16bit_storage : require
#extension GL_EXT_debug_printf : require

layout(location = 0) rayPayloadInEXT vec4 payload;

hitAttributeEXT vec2 attributes;

layout(binding = 2, set = 0) buffer Indices
{
    uint16_t indices[];
};
layout(binding = 3, set = 0, scalar) buffer Vertices
{
    vec3 vertices[];
};

void main()
{
    const int primitiveID = gl_PrimitiveID;
    const uint i0 = uint(indices[3 * primitiveID]);
    const vec3 v0 = vertices[i0];
    debugPrintfEXT("primitive id %d\n", primitiveID);

    payload = vec4(vec3(0.5) + 0.25 * v0, 1.0);
}