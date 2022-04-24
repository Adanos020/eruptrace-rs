#version 450

layout(location = 0) in vec4 fColor;
layout(location = 1) in vec2 fTexCoords;

layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 0) uniform sampler2D fontTexture;

void main() {
    outColor = fColor * texture(fontTexture, fTexCoords);
}
