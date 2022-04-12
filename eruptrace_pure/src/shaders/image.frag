#version 450

#define RENDER_NORMALS 0

// Math constants ------------------------------------------------------------------------------------------------------

const float FLOAT_MAX = 3.402823466e+38f;
const float FLOAT_MIN = 1.175494351e-38f;
const float EPSILON = 1e-4f;
const float PI = 3.1415926535897932384626433832795f;
const float TWO_PI = 2.f * PI;
const float HALF_PI = 0.5f * PI;
const float ONE_OVER_PI = 1.f / PI;
const float ONE_OVER_TWO_PI = 1.f / (2.f * PI);

// Types ---------------------------------------------------------------------------------------------------------------

const uint MATERIAL_DIFFUSIVE = 0;
const uint MATERIAL_REFLECTIVE = 1;
const uint MATERIAL_REFRACTIVE = 2;
const uint MATERIAL_EMITTING = 3;

const uint BIH_BRANCH_X = 0;
const uint BIH_BRANCH_Y = 1;
const uint BIH_BRANCH_Z = 2;
const uint BIH_LEAF = 3;

// Structs -------------------------------------------------------------------------------------------------------------

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

// I/O -----------------------------------------------------------------------------------------------------------------

layout(location = 0) out vec4 fragColour;

layout(set = 0, binding = 0) uniform sampler2DArray textures;
layout(set = 0, binding = 1) uniform sampler2DArray normalMaps;
layout(set = 0, binding = 2) uniform CameraUniform {
    vec4 position;
    vec4 horizontal;
    vec4 vertical;
    vec4 bottomLeft;
    vec2 imgSize;
    vec2 imgSizeInv;
    uint sqrtSamples;
    uint maxReflections;
} camera;
layout(set = 0, binding = 3, std140) readonly buffer BIH {
    BihNode bihNodes[];
};
layout(set = 0, binding = 4, std140) readonly buffer MaterialData {
    Material materials[];
};
layout(set = 0, binding = 5, std140) readonly buffer TriangleData {
    Triangle triangles[];
};

layout(push_constant) uniform Constants {
    uint nTriangles;
    bool bUseBih;
};

// Utils ---------------------------------------------------------------------------------------------------------------

float rand(float at) {
    return fract(sin(dot((at + gl_FragCoord.xy), vec2(12.9898f, 78.233f))) * 43758.5453123f);
}

vec3 randPointInUnitCube(float at) {
    return vec3(
        -1.f + (2.f * rand(at)),
        -1.f + (2.f * rand(at + 1)),
        -1.f + (2.f * rand(at + 2))
    );
}

// Taken from: https://github.com/LWJGL/lwjgl3-demos/blob/main/res/org/lwjgl/demo/opengl/raytracing/randomCommon.glsl
vec3 randDirection(float at) {
    vec3 randPoint = randPointInUnitCube(at);
    float ang1 = (randPoint.x + 1.0) * PI;
    float u = randPoint.y;
    float u2 = u * u;
    float sqrt1MinusU2 = sqrt(1.0 - u2);
    float x = sqrt1MinusU2 * cos(ang1);
    float y = sqrt1MinusU2 * sin(ang1);
    float z = u;
    return vec3(x, y, z);
}

vec3 pointOnRay(in Ray ray, float distance) {
    return ray.origin + (ray.direction * distance);
}

vec2 mappingOnUnitSphere(vec3 pointOnSphere) {
    return vec2(
        1.f - ((atan(pointOnSphere.z, pointOnSphere.x) + PI) * ONE_OVER_TWO_PI),
        1.f - ((asin(pointOnSphere.y) + HALF_PI) * ONE_OVER_PI)
    );
}

vec4 sampleTexture(vec2 texCoords, uint textureIndex) {
    return texture(textures, vec3(texCoords, textureIndex));
}

vec3 sampleNormalMap(vec2 texCoords, uint normalMapIndex) {
    vec3 normal = texture(normalMaps, vec3(texCoords, normalMapIndex)).xyz;
    return (normal * 2.f) - 1.f;
}

vec3 mapNormal(vec3 worldNormal, vec3 mappedNormal) {
    vec3 t = cross(worldNormal, vec3(0.f, 1.f, 0.f));
    if (dot(t, t) == 0.f) {
        t = cross(worldNormal, vec3(0.f, 0.f, 1.f));
    }
    vec3 b = cross(worldNormal, t);
    mat3 transform = mat3(t, b, worldNormal);
    return normalize(transform * mappedNormal);
}

// Ray tracing ---------------------------------------------------------------------------------------------------------

vec4 trace(Ray ray);

bool hitShapeBih(in Ray ray, out Hit hit);
bool hitShapeBruteforce(in Ray ray, out Hit hit);
bool hitTriangle(in Ray ray, in Triangle triangle, float distMin, float distMax, out Hit hit);

bool scatter(Hit hit, out Scattering scattering);
bool scatterDiffusive(in Hit hit, in Material mat, out Scattering scattering);
bool scatterReflective(in Hit hit, in Material mat, out Scattering scattering);
bool scatterRefractive(in Hit hit, in Material mat, out Scattering scattering);
bool scatterEmitting(in Hit hit, in Material mat, out Scattering scattering);

void main() {
    Ray ray;
    ray.origin = camera.position.xyz;
    vec4 pixelColor = vec4(0.f);
    uint samples = camera.sqrtSamples * camera.sqrtSamples;
    for (int i = 0; i < samples; ++i) {
        float u = (gl_FragCoord.x + rand(i)) * camera.imgSizeInv.x;
        float v = (camera.imgSize.y - gl_FragCoord.y + rand(i + 0.5f)) * camera.imgSizeInv.y;
        vec4 samplePosition = camera.bottomLeft + (u * camera.horizontal) + (v * camera.vertical);
        ray.direction = (samplePosition - camera.position).xyz;
        ray.invDirection = 1.f / ray.direction;
        pixelColor += trace(ray);
    }
    fragColour = sqrt(pixelColor / float(samples));
}

vec4 trace(Ray ray) {
    vec4 finalColor = vec4(1.f);
    for (int iReflection = 0; iReflection < camera.maxReflections; ++iReflection) {
        Hit hit;
        if (bUseBih ? hitShapeBih(ray, hit) : hitShapeBruteforce(ray, hit)) {
            Scattering scattering;
            bool bScattered = scatter(hit, scattering);
            finalColor *= scattering.color;
            if (bScattered) {
                ray = scattering.newRay;
            } else {
                // Emission
                break;
            }
        } else {
            // Sky
            vec3 rayDir = normalize(ray.direction);
            finalColor *= sampleTexture(mappingOnUnitSphere(rayDir), 0);
            break;
        }
    }
    return finalColor;
}

bool hitShapeBih(in Ray ray, out Hit hit) {
    bool hitOccured = false;
    float minDistance = EPSILON;
    float maxDistance = FLOAT_MAX;

    struct StackEntry {
        uint nodeIndex;
        float minDistance;
        float maxDistance;
    } stack[64];
    stack[0] = StackEntry(0, minDistance, maxDistance);
    int entryIndex = 0;

    while (entryIndex >= 0) {
        StackEntry currEntry = stack[entryIndex--];
        bool leafHit = true;

        // Traverse tree
        while (bihNodes[currEntry.nodeIndex].nodeType != BIH_LEAF) {
            uint axis = bihNodes[currEntry.nodeIndex].nodeType;
            vec2 distancesToPlanes = vec2(bihNodes[currEntry.nodeIndex].clipLeft, bihNodes[currEntry.nodeIndex].clipRight);
            distancesToPlanes = (distancesToPlanes - ray.origin[axis]) * ray.invDirection[axis];

            uint node1 = uint(ray.direction[axis] < 0);
            uint node2 = 1 - node1;

            float dist1 = distancesToPlanes[node1];
            float dist2 = distancesToPlanes[node2];

            bool hit1Occurred = dist1 >= currEntry.minDistance;
            bool hit2Occurred = dist2 <= currEntry.maxDistance;

            uint children[2] = uint[](
                bihNodes[currEntry.nodeIndex].childLeft,
                bihNodes[currEntry.nodeIndex].childRight
            );

            if (hit1Occurred) {
                currEntry.nodeIndex = children[node1];
                currEntry.maxDistance = min(currEntry.maxDistance, dist1);
                if (hit2Occurred) {
                    stack[++entryIndex].nodeIndex = children[node2];
                    stack[entryIndex].minDistance = max(currEntry.minDistance, dist2);
                    stack[entryIndex].maxDistance = currEntry.maxDistance;
                }
            } else if (hit2Occurred) {
                currEntry.nodeIndex = children[node2];
                currEntry.maxDistance = max(currEntry.minDistance, dist2);
            } else {
                leafHit = false;
                break;
            }
        }

        // Ray-triangle intersection
        if (leafHit) {
            uint triangleIndex = bihNodes[currEntry.nodeIndex].childLeft;
            uint triangleCount = bihNodes[currEntry.nodeIndex].childRight;
            for (uint i = triangleIndex; i < triangleIndex + triangleCount; ++i) {
                Hit tempHit;
                if (hitTriangle(ray, triangles[i], minDistance, maxDistance, tempHit)) {
                    hitOccured = true;
                    maxDistance = tempHit.distance;
                    hit = tempHit;
                }
            }
        }
    }

    return hitOccured;
}

bool hitShapeBruteforce(in Ray ray, out Hit hit) {
    bool hitOccured = false;
    const float minDistance = EPSILON;
    float maxDistance = FLOAT_MAX;

    for (int i = 0; i < nTriangles; ++i) {
        Hit tempHit;
        if (hitTriangle(ray, triangles[i], minDistance, maxDistance, tempHit)) {
            hitOccured = true;
            maxDistance = tempHit.distance;
            hit = tempHit;
        }
    }

    return hitOccured;
}

// Möller-Trumbore algorithm
bool hitTriangle(in Ray ray, in Triangle triangle, float distMin, float distMax, out Hit hit) {
    vec3 edge1 = triangle.positions[1] - triangle.positions[0];
    vec3 edge2 = triangle.positions[2] - triangle.positions[0];
    vec3 p = cross(ray.direction, edge2);
    float determinant = dot(edge1, p);

    if (abs(determinant) < EPSILON) {
        return false;
    }

    float determinantInv = 1.f / determinant;
    vec3 t = ray.origin - triangle.positions[0];
    vec3 q = cross(t, edge1);
    float u = dot(t, p) * determinantInv;
    float v = dot(ray.direction, q) * determinantInv;

    if (u < 0.f || u > 1.f || v < 0.f || u + v > 1.f) {
        return false;
    }

    float distance = dot(q, edge2) * determinantInv;

    if (distance < distMin || distance > distMax) {
        return false;
    }

    vec3 normal = ((1.f - u - v) * triangle.normals[0]) + (u * triangle.normals[1]) + (v * triangle.normals[2]);
    float dotRayNorm = dot(ray.direction, normal);

    hit.distance = distance;
    hit.position = pointOnRay(ray, distance);
    hit.incidental = ray.direction;
    hit.normal = normal * -sign(dotRayNorm);
    hit.texCoords = ((1.f - u - v) * triangle.texcoords[0]) + (u * triangle.texcoords[1]) + (v * triangle.texcoords[2]);
    hit.bFrontFace = dotRayNorm < 0.f;
    hit.materialIndex = triangle.materialIndex;

    return true;
}

bool scatter(Hit hit, out Scattering scattering) {
    Material material = materials[hit.materialIndex];

    vec3 mappedNormal = sampleNormalMap(hit.texCoords, material.normalMapIndex);
    hit.normal = mapNormal(hit.normal, mappedNormal);

#if RENDER_NORMALS
    scattering.color = vec4(0.5f + (0.5f * hit.normal), 1.f);
    return false;
#endif

    switch (material.materialType) {
        case MATERIAL_DIFFUSIVE: {
            return scatterDiffusive(hit, material, scattering);
        }
        case MATERIAL_REFLECTIVE: {
            return scatterReflective(hit, material, scattering);
        }
        case MATERIAL_REFRACTIVE: {
            return scatterRefractive(hit, material, scattering);
        }
        case MATERIAL_EMITTING: {
            return scatterEmitting(hit, material, scattering);
        }
        default: {
            return false;
        }
    }
}

bool scatterDiffusive(in Hit hit, in Material material, out Scattering scattering) {
    vec3 scatterDirection = randDirection(dot(hit.position, gl_FragCoord.xyz));
    scatterDirection *= sign(dot(scatterDirection, hit.normal));
    scatterDirection = normalize(scatterDirection);
    scattering.newRay = Ray(hit.position, scatterDirection, 1.f / scatterDirection);
    scattering.color = sampleTexture(hit.texCoords, material.textureIndex);
    return true;
}

bool scatterReflective(in Hit hit, in Material material, out Scattering scattering) {
    float fuzz = material.parameter;
    vec3 reflected = reflect(hit.incidental, hit.normal);
    vec3 randDir = randDirection(dot(hit.position, gl_FragCoord.xyz));
    vec3 scatterDirection = reflected + (fuzz * randDir);
    scatterDirection *= sign(dot(scatterDirection, hit.normal));
    if (dot(scatterDirection, hit.normal) > 0.f) {
        scattering.color = sampleTexture(hit.texCoords, material.textureIndex);
        scattering.newRay = Ray(hit.position, scatterDirection, 1.f / scatterDirection);
        return true;
    }
    return false;
}

bool scatterRefractive(in Hit hit, in Material material, out Scattering scattering) {
    float refractiveIndex = hit.bFrontFace ? (1.f / material.parameter) : material.parameter;
    vec3 direction = normalize(hit.incidental);
    float cosTheta = min(dot(-direction, hit.normal), 1.f);
    float sinTheta = sqrt(1.f - (cosTheta * cosTheta));
    bool cannotRefract = refractiveIndex * sinTheta > 1.f;

    float reflectance = (1.f - refractiveIndex) / (1.f + refractiveIndex);
    reflectance *= reflectance;
    reflectance += (1.f - reflectance) * pow((1.f - cosTheta), 5.f);
    bool shouldReflect = reflectance > rand(dot(hit.position, gl_FragCoord.xyz));

    vec3 scatterDirection;
    if (cannotRefract || shouldReflect) {
        scatterDirection = reflect(direction, hit.normal);
    } else {
        scatterDirection = refract(direction, hit.normal, refractiveIndex);
    }

    scattering.color = sampleTexture(hit.texCoords, material.textureIndex);
    scattering.newRay = Ray(hit.position, scatterDirection, 1.f / scatterDirection);
    return true;
}

bool scatterEmitting(in Hit hit, in Material material, out Scattering scattering) {
    float intensity = material.parameter;
    scattering.color = intensity * sampleTexture(hit.texCoords, material.textureIndex);
    return false;
}
