#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 texCoord;
layout(location = 2) in vec4 color;

layout(location = 0) out vec4 fColor;
layout(location = 1) out vec2 fTexCoord;

layout(push_constant) uniform Constants {
    vec2 screenSize;
};

void main() {
    gl_Position = vec4(2.f * position / screenSize - 1.f, 0.f, 1.f);
    fColor = color;
    fTexCoord = texCoord;
}
