#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 texCoord;

layout(location = 0) out vec3 fPosition;
layout(location = 1) out vec3 fNormal;
layout(location = 2) out vec2 fTexCoord;

struct MeshMeta {
    mat4 modlTransform;
    uint materialIndex;
};

layout(set = 0, binding = 0, std140) readonly buffer MeshMetas {
    MeshMeta meshMetas[];
};
layout(set = 1, binding = 0, std140) readonly uniform CameraUniforms {
    mat4 viewTransform;
    mat4 projTransform;
};
layout(push_constant, std140) readonly uniform Constants {
    uint meshMetaIndex;
};

void main() {
    mat4 modlTransform = meshMetas[meshMetaIndex].modlTransform;
    gl_Position = projTransform * viewTransform * modlTransform * vec4(position, 1.f);
    fPosition = vec3(modlTransform * vec4(position, 1.f));
    fNormal = mat3(transpose(inverse(modlTransform))) * normal;
    fTexCoord = texCoord;
}
