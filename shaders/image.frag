#version 450

const float FLOAT_MAX = 3.402823466e+38f;
const float FLOAT_MIN = 1.175494351e-38f;
const float PI = 3.1415926535897932384626433832795f;
const float TWO_PI = 2.f * PI;

// Material types
const uint MATERIAL_REFLECTIVE = 0;
const uint MATERIAL_REFRACTIVE = 1;
const uint MATERIAL_EMISSIVE = 2;

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

struct Sphere {
    vec3 position;
    float radius;
    uint materialType;
    uint materialIndex;
};

struct ReflectiveMaterial {
    vec4 color;
    float fuzz;
};

struct Ray {
    vec3 position;
    vec3 direction;
};

struct Hit {
    vec3 position;
    vec3 incidental;
    vec3 normal;
    float distance;
    uint materialType;
    uint materialIndex;
};

struct Scattering {
    Ray newRay;
    vec4 color;
};

layout(location = 0) out vec4 fragColour;

layout(set = 0, binding = 0) uniform CameraUniform {
    Camera camera;
};
layout(set = 0, binding = 1) buffer ShapesData {
    float shapeValues[];
};
layout(set = 0, binding = 2) buffer MaterialData {
    float materialValues[];
};

float rand(float at);
vec3 randPointInUnitCube(float at);
vec3 randPointInUnitSphere(float at);
vec3 randPointOnUnitSphere(float at);

vec3 pointOnRay(in Ray ray, float t);

Sphere sphereAt(inout uint iShapeValue);

ReflectiveMaterial reflectiveAt(inout uint iMaterialValue);

vec4 trace(Ray ray);

bool hitShape(in Ray ray, out Hit hit);
bool hitSphere(in Ray ray, in Sphere sphere, float distMin, float distMax, out Hit hit);

bool scatter(in Hit hit, out Scattering scattering);
bool scatterReflective(in Hit hit, in ReflectiveMaterial mat, out Scattering scattering);

void main() {
    Ray ray;
    ray.position = camera.position.xyz;
    vec4 color = vec4(0.f);
    for (int i = 0; i < camera.samples; ++i) {
        float u = (gl_FragCoord.x + rand(i)) * camera.imgSizeInv.x;
        float v = (camera.imgSize.y - gl_FragCoord.y + rand(i + 0.5f)) * camera.imgSizeInv.y;
        vec3 samplePosition = (camera.bottomLeft + (u * camera.horizontal) + (v * camera.vertical)).xyz;
        ray.direction = samplePosition - camera.position.xyz;
        color += trace(ray);
    }
    fragColour = sqrt(color / float(camera.samples));
}

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

vec3 randPointOnUnitSphere(float at) {
    return normalize(randPointInUnitCube(at));
}

vec3 pointOnRay(in Ray ray, float distance) {
    return ray.position + (ray.direction * distance);
}

Sphere sphereAt(inout uint iShapeValue) {
    return Sphere(
        // Position
        vec3(shapeValues[iShapeValue++],
             shapeValues[iShapeValue++],
             shapeValues[iShapeValue++]),
        // Radius
        shapeValues[iShapeValue++],
        // Material type
        uint(shapeValues[iShapeValue++]),
        // Material index
        uint(shapeValues[iShapeValue++])
    );
}

ReflectiveMaterial reflectiveAt(inout uint iMaterialValue) {
    return ReflectiveMaterial(
        // Colour
        vec4(materialValues[iMaterialValue++],
             materialValues[iMaterialValue++],
             materialValues[iMaterialValue++],
             materialValues[iMaterialValue++]),
        // Fuzz
        materialValues[iMaterialValue++]
    );
}

vec4 trace(Ray ray) {
    vec4 finalColor = vec4(1.f);
    for (int i = 0; i < camera.maxReflections; ++i) {
        Hit hit;
        if (hitShape(ray, hit)) {
            Scattering scattering;
            if (scatter(hit, scattering)) {
                ray = scattering.newRay;
                finalColor *= 0.5f * scattering.color;
            }
        } else {
            float factor = 0.5f * (normalize(ray.direction).y + 1.f);
            finalColor *= vec4(mix(vec3(1.f), vec3(0.5f, 0.7f, 1.f), factor), 1.f);
            break;
        }
    }
    return finalColor;
}

bool hitShape(in Ray ray, out Hit hit) {
    bool hitOccured = false;
    uint iShapeValue = 0;

    float minDistance = 0.0001f;
    float maxDistance = FLOAT_MAX;

    float nSpheres = shapeValues[iShapeValue++];
    for (float iSphere = 0.f; iSphere < nSpheres; ++iSphere) {
        Sphere sphere = sphereAt(iShapeValue);
        Hit tempHit;
        if (hitSphere(ray, sphere, minDistance, maxDistance, tempHit)) {
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

    if (discriminant > 0.f) {
        float aInv = 1.f / a;
        float sqrtDis = sqrt(discriminant);

        float root = (-b - sqrtDis) * aInv;
        if (root < distMin || root > distMax) {
            root = (-b + sqrtDis) * aInv;
            if (root < distMin || root > distMax) {
                return false;
            }
        }

        hit.distance = root;
        hit.position = pointOnRay(ray, root);
        hit.incidental = ray.direction;
        hit.normal = (hit.position - sphere.position) / sphere.radius;
        hit.normal *= sign(dot(ray.direction, hit.normal));
        hit.materialType = sphere.materialType;
        hit.materialIndex = sphere.materialIndex;
        return true;
    }
    return false;
}

bool scatter(in Hit hit, out Scattering scattering) {
    switch (hit.materialType) {
        case MATERIAL_REFLECTIVE: {
            uint iMaterialValue = 5 * hit.materialIndex;
            ReflectiveMaterial reflectiveMat = reflectiveAt(iMaterialValue);
            return scatterReflective(hit, reflectiveMat, scattering);
        }
        case MATERIAL_REFRACTIVE: {
            return false;
        }
        case MATERIAL_EMISSIVE: {
            return false;
        }
        default: {
            return false;
        }
    }
}

bool scatterReflective(in Hit hit, in ReflectiveMaterial mat, out Scattering scattering) {
//    vec3 reflected = reflect(hit.incidental, hit.normal);
//    vec3 newDir = reflected + (mat.fuzz * randPointOnUnitSphere(dot(hit.incidental.xy, hit.incidental.zx)));
//    if (dot(newDir, hit.normal) > 0.f) {
//        scattering.color = mat.color;
//        scattering.newRay = Ray(hit.position, newDir);
//        return true;
//    }
//    return false;
    vec3 target = hit.position + hit.normal + randPointOnUnitSphere(dot(hit.position, hit.position));
    scattering.newRay = Ray(hit.position, target - hit.position);
    scattering.color = mat.color;
    return true;
}
