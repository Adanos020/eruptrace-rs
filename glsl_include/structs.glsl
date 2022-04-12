#ifndef TYPES
#define TYPES

struct BihNode {
    uint nodeType;
    uint childLeft;
    uint childRight;
    float clipLeft;
    float clipRight;
};

struct Triangle {
    vec3 positions[3];
    vec3 normals[3];
    vec2 texcoords[3];
    uint materialIndex;
};

struct Material {
    uint materialType;
    uint textureIndex;
    uint normalMapIndex;
    float parameter;
};

struct Ray {
    vec3 origin;
    vec3 direction;
    vec3 invDirection;
};

struct Hit {
    vec3 position;
    vec3 incidental;
    vec3 normal;
    vec2 texCoords;
    float distance;
    uint materialIndex;
    bool bFrontFace;
};

struct Scattering {
    Ray newRay;
    vec4 color;
};

#endif // TYPES
