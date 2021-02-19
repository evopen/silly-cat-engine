#version 460 core
#extension GL_EXT_ray_tracing : require

layout(location = 0) rayPayloadInEXT vec4 payload;

hitAttributeEXT vec2 attributes;

void main()
{
    const int primitiveID = gl_PrimitiveID;
    payload = vec4(primitiveID / 10.0, primitiveID / 100.0, primitiveID / 1000.0, 1.0);
}