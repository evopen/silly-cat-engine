#version 460

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform texture2D t_texture;
layout(set = 0, binding = 1) uniform sampler s_texture;

void main()
{
    out_color = texture(sampler2D(t_texture, s_texture), in_uv);
}
