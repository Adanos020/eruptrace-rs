#version 450

layout(location = 0) in vec3 fPosition;
layout(location = 1) in vec3 fNormal;
layout(location = 2) in vec2 fTexCoord;

layout(location = 0) out vec4 fragColor;

struct MeshMeta {
    mat4 modlTransform;
    uint materialIndex;
};

layout(set = 0, binding = 0, std140) readonly buffer MeshMetas {
    MeshMeta meshMetas[];
};
layout(push_constant, std140) readonly uniform Constants {
    uint meshMetaIndex;
};

void main() {
    meshMetas[meshMetaIndex];
    vec3 normal = normalize(fNormal);
    fragColor = vec4(0.5f * (1.f + normal), 1.f);
}
