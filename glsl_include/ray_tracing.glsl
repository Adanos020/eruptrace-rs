#ifndef RAY_TRACING
#define RAY_TRACING

#include <constants.glsl>
#include <structs.glsl>
#include <utils.glsl>

// Types ---------------------------------------------------------------------------------------------------------------

const uint MATERIAL_DIFFUSIVE = 0;
const uint MATERIAL_REFLECTIVE = 1;
const uint MATERIAL_REFRACTIVE = 2;
const uint MATERIAL_EMITTING = 3;

const uint BIH_BRANCH_X = 0;
const uint BIH_BRANCH_Y = 1;
const uint BIH_BRANCH_Z = 2;
const uint BIH_LEAF = 3;

const uint FLAG_USE_BIH = 1 << 0;
const uint FLAG_RENDER_NORMALS = 1 << 1;
const uint FLAG_RENDER_BIH = 1 << 2;

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

vec4 trace(Ray ray) {
    vec4 finalColor = vec4(1.f);
    for (int iReflection = 0; iReflection < camera.maxReflections; ++iReflection) {
        Hit hit;
        if ((flags & FLAG_USE_BIH) != 0 ? hitShapeBih(ray, hit) : hitShapeBruteforce(ray, hit)) {
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
    stack[0].nodeIndex = 0;
    stack[0].minDistance = minDistance;
    stack[0].maxDistance = maxDistance;

    uint level = 0;

    int entryIndex = 0;
    while (entryIndex >= 0) {
        StackEntry currEntry = stack[entryIndex--];
        bool leafHit = (flags & FLAG_RENDER_BIH) == 0;

        // Traverse subtree
        while (bihNodes[currEntry.nodeIndex].nodeType != BIH_LEAF) {
            uint axis = bihNodes[currEntry.nodeIndex].nodeType;
            float distancesToPlanes[2] = float[](
                (bihNodes[currEntry.nodeIndex].clipLeft - ray.origin[axis]) * ray.invDirection[axis],
                (bihNodes[currEntry.nodeIndex].clipRight - ray.origin[axis]) * ray.invDirection[axis]
            );

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

            if ((flags & FLAG_RENDER_BIH) != 0 && currEntry.nodeIndex == drawBihLevel) {
                if (hit1Occurred) {
                    if (hit2Occurred) {
                        hit.materialIndex = 3;
                        hit.distance = min(dist1, dist2);
                        hit.position = pointOnRay(ray, hit.distance);
                    } else {
                        hit.materialIndex = 0;
                        hit.distance = dist1;
                        hit.position = pointOnRay(ray, hit.distance);
                    }
                    return true;
                } else if (hit2Occurred) {
                    hit.materialIndex = 1;
                    hit.distance = dist2;
                    hit.position = pointOnRay(ray, hit.distance);
                    return true;
                }
            }

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
                currEntry.minDistance = max(currEntry.minDistance, dist2);
            } else {
                leafHit = false;
                break;
            }
        }

        // Ray-triangle intersection
        if (leafHit && (flags & FLAG_RENDER_BIH) == 0) {
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

    if ((flags & FLAG_RENDER_BIH) != 0) {
        hit.materialIndex = 2;
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

    if ((flags & FLAG_RENDER_NORMALS) != 0) {
        scattering.color = vec4(0.5f + (0.5f * hit.normal), 1.f);
        return false;
    } else if ((flags & FLAG_RENDER_BIH) != 0) {
        scattering.color = vec4(
            float(hit.materialIndex == 0 || hit.materialIndex == 3),
            float(hit.materialIndex == 2 || hit.materialIndex == 3),
            float(hit.materialIndex == 1),
            1.f);
        return false;
    } else {
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

#endif // RAY_TRACING
