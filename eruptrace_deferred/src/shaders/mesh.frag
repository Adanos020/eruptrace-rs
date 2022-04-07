#version 450

layout(location = 0) in vec3 fPosition;
layout(location = 1) in vec3 fNormal;
layout(location = 2) in vec2 fTexCoord;

layout(location = 0) out vec4 outPosition;
layout(location = 1) out vec4 outNormal;
layout(location = 2) out vec4 outTexCoord;

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
    outPosition = vec4(fPosition, 1.f);
    outNormal   = vec4(normalize(fNormal), 1.f);
    outTexCoord = vec4(fTexCoord, meshMetas[meshMetaIndex].materialIndex, 0.f);
}
