#version 450

// Math constants ------------------------------------------------------------------------------------------------------

const float FLOAT_MAX = 3.402823466e+38f;
const float FLOAT_MIN = 1.175494351e-38f;
const float EPSILON = 1e-5;
const float PI = 3.1415926535897932384626433832795f;
const float TWO_PI = 2.f * PI;
const float HALF_PI = 0.5f * PI;
const float ONE_OVER_PI = 1.f / PI;
const float ONE_OVER_TWO_PI = 1.f / (2.f * PI);

// Material types ------------------------------------------------------------------------------------------------------

const uint MATERIAL_DIFFUSIVE = 0;
const uint MATERIAL_REFLECTIVE = 1;
const uint MATERIAL_REFRACTIVE = 2;
const uint MATERIAL_EMITTING = 3;

// Structs -------------------------------------------------------------------------------------------------------------

struct Camera {
    vec4 position;
    vec4 horizontal;
    vec4 vertical;
    vec4 bottomLeft;
    vec2 imgSize;
    vec2 imgSizeInv;
    uint samples;
    uint maxReflections;
};

struct Vertex {
    vec3 position;
    vec3 normal;
    vec2 texCoords;
};

struct Sphere {
    vec3 position;
    float radius;
    uint materialType;
    uint materialIndex;
};

struct Triangle {
    Vertex a;
    Vertex b;
    Vertex c;
    uint materialType;
    uint materialIndex;
};

struct Material {
    uint textureIndex;
    float parameter;
};

struct Ray {
    vec3 position;
    vec3 direction;
};

struct Hit {
    vec3 position;
    vec3 incidental;
    vec3 normal;
    vec2 texCoords;
    float distance;
    uint materialType;
    uint materialIndex;
    bool bFrontFace;
};

struct Scattering {
    Ray newRay;
    vec4 color;
};

// I/O -----------------------------------------------------------------------------------------------------------------

layout(location = 0) out vec4 fragColour;

layout(set = 0, binding = 0) uniform CameraUniform {
    Camera camera;
};
layout(set = 0, binding = 1) readonly buffer ShapesData {
    float shapeValues[];
};
layout(set = 0, binding = 2, std140) readonly buffer MaterialData {
    Material materials[];
};
layout(set = 0, binding = 3) uniform sampler2DArray textures;

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
    return ray.position + (ray.direction * distance);
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

Sphere sphereAt(inout uint iShapeValue) {
    Sphere s;
    s.position      = vec3(shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++]);
    s.radius        = shapeValues[iShapeValue++];
    s.materialType  = uint(shapeValues[iShapeValue++]);
    s.materialIndex = uint(shapeValues[iShapeValue++]);
    return s;
}

Triangle triangleAt(inout uint iShapeValue) {
    Triangle t;
    t.a.position  = vec3(shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++]);
    t.a.normal    = vec3(shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++]);
    t.a.texCoords = vec2(shapeValues[iShapeValue++], shapeValues[iShapeValue++]);

    t.b.position  = vec3(shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++]);
    t.b.normal    = vec3(shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++]);
    t.b.texCoords = vec2(shapeValues[iShapeValue++], shapeValues[iShapeValue++]);

    t.c.position  = vec3(shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++]);
    t.c.normal    = vec3(shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++]);
    t.c.texCoords = vec2(shapeValues[iShapeValue++], shapeValues[iShapeValue++]);

    t.materialType  = uint(shapeValues[iShapeValue++]);
    t.materialIndex = uint(shapeValues[iShapeValue++]);
    return t;
}

// Ray tracing ---------------------------------------------------------------------------------------------------------

vec4 trace(Ray ray);

bool hitShape(in Ray ray, out Hit hit);
bool hitSphere(in Ray ray, in Sphere sphere, float distMin, float distMax, out Hit hit);
bool hitTriangle(in Ray ray, in Triangle triangle, float distMin, float distMax, out Hit hit);

bool scatter(in Hit hit, out Scattering scattering);
bool scatterDiffusive(in Hit hit, in Material mat, out Scattering scattering);
bool scatterReflective(in Hit hit, in Material mat, out Scattering scattering);
bool scatterRefractive(in Hit hit, in Material mat, out Scattering scattering);
bool scatterEmitting(in Hit hit, in Material mat, out Scattering scattering);

void main() {
    Ray ray;
    ray.position = camera.position.xyz;
    vec4 pixelColor = vec4(0.f);
    for (int i = 0; i < camera.samples; ++i) {
        float u = (gl_FragCoord.x + rand(i)) * camera.imgSizeInv.x;
        float v = (camera.imgSize.y - gl_FragCoord.y + rand(i + 0.5f)) * camera.imgSizeInv.y;
        vec4 samplePosition = camera.bottomLeft + (u * camera.horizontal) + (v * camera.vertical);
        ray.direction = samplePosition.xyz - camera.position.xyz;
        pixelColor += trace(ray);
    }
    fragColour = sqrt(pixelColor / float(camera.samples));
}

vec4 trace(Ray ray) {
    vec4 finalColor = vec4(1.f);
    for (int iReflection = 0; iReflection < camera.maxReflections; ++iReflection) {
        Hit hit;
        if (hitShape(ray, hit)) {
            Scattering scattering;
            if (scatter(hit, scattering)) {
                // Scattering
                finalColor *= scattering.color;
                ray = scattering.newRay;
            } else {
                // Emission
                finalColor *= scattering.color;
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

bool hitShape(in Ray ray, out Hit hit) {
    bool hitOccured = false;
    uint iShapeValue = 0;
    const float minDistance = EPSILON;
    float maxDistance = FLOAT_MAX;

    int nSpheres = int(shapeValues[iShapeValue++]);
    for (int iSphere = 0; iSphere < nSpheres; ++iSphere) {
        Hit tempHit;
        Sphere sphere = sphereAt(iShapeValue);
        if (hitSphere(ray, sphere, minDistance, maxDistance, tempHit)) {
            hitOccured = true;
            maxDistance = tempHit.distance;
            hit = tempHit;
        }
    }

    int nTriangles = int(shapeValues[iShapeValue++]);
    for (int iTriangle = 0; iTriangle < nTriangles; ++iTriangle) {
        Hit tempHit;
        Triangle triangle = triangleAt(iShapeValue);
        if (hitTriangle(ray, triangle, minDistance, maxDistance, tempHit)) {
            hitOccured = true;
            maxDistance = tempHit.distance;
            hit = tempHit;
        }
    }

    return hitOccured;
}

bool hitSphere(in Ray ray, in Sphere sphere, float distMin, float distMax, out Hit hit) {
    vec3 rs = ray.position - sphere.position;
    float a = dot(ray.direction, ray.direction);
    float b = dot(rs, ray.direction);
    float c = dot(rs, rs) - (sphere.radius * sphere.radius);

    float discriminant = (b * b) - (a * c);
    if (discriminant < 0.f) {
        return false;
    }

    float sqrtDis = sqrt(discriminant);
    float aInv = 1.f / a;
    float root = (-b - sqrtDis) * aInv;

    if (root < distMin || root > distMax) {
        root = (-b + sqrtDis) * aInv;
        if (root < distMin || root > distMax) {
            return false;
        }
    }

    vec3 hitPosition = pointOnRay(ray, root);
    vec3 normal = (hitPosition - sphere.position) / sphere.radius;
    float dotRayNorm = dot(ray.direction, normal);

    hit.distance = root;
    hit.position = hitPosition;
    hit.normal = normal * -sign(dotRayNorm);
    hit.incidental = ray.direction;
    hit.texCoords = mappingOnUnitSphere(normalize(hitPosition - sphere.position));
    hit.materialType = sphere.materialType;
    hit.materialIndex = sphere.materialIndex;
    hit.bFrontFace = dotRayNorm < 0.f;
    return true;
}

// Möller-Trumbore algorithm
bool hitTriangle(in Ray ray, in Triangle triangle, float distMin, float distMax, out Hit hit) {
    vec3 edge1 = triangle.b.position - triangle.a.position;
    vec3 edge2 = triangle.c.position - triangle.a.position;
    vec3 p = cross(ray.direction, edge2);
    float determinant = dot(edge1, p);

    if (determinant < EPSILON) {
        return false;
    }

    float determinantInv = 1.f / determinant;
    vec3 t = ray.position - triangle.a.position;
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

    vec3 normal = ((1.f - u - v) * triangle.a.normal) + (u * triangle.b.normal) + (v * triangle.c.normal);
    float dotRayNorm = dot(ray.direction, normal);

    hit.distance = distance;
    hit.position = pointOnRay(ray, distance);
    hit.incidental = ray.direction;
    hit.normal = normal * -sign(dotRayNorm);
    hit.texCoords = ((1.f - u - v) * triangle.a.texCoords) + (u * triangle.b.texCoords) + (v * triangle.c.texCoords);
    hit.materialType = triangle.materialType;
    hit.materialIndex = triangle.materialIndex;
    hit.bFrontFace = dotRayNorm < 0.f;

    return true;
}

bool scatter(in Hit hit, out Scattering scattering) {
    Material material = materials[hit.materialIndex];
    switch (hit.materialType) {
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
    scattering.newRay = Ray(hit.position, normalize(scatterDirection));
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
        scattering.newRay = Ray(hit.position, scatterDirection);
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
    bool shouldReflect = reflectance > rand(dot(hit.position, hit.incidental));

    vec3 scatterDirection = (cannotRefract || shouldReflect)
        ? reflect(direction, hit.normal)
        : refract(direction, hit.normal, refractiveIndex);

    scattering.color = sampleTexture(hit.texCoords, material.textureIndex);
    scattering.newRay = Ray(hit.position, scatterDirection);
    return true;
}

bool scatterEmitting(in Hit hit, in Material material, out Scattering scattering) {
    float intensity = material.parameter;
    scattering.color = intensity * sampleTexture(hit.texCoords, material.textureIndex);
    return false;
}
