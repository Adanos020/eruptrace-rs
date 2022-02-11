#version 450

#define DRAW_NORMALS 1

const float FLOAT_MAX = 3.402823466e+38;
const float FLOAT_MIN = 1.175494351e-38;

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
    vec4 color;
    vec3 position;
    float radius;
};

struct Hit {
    vec3 position;
    vec3 normal;
    float distance;
};

struct Ray {
    vec3 position;
    vec3 direction;
};

layout(location = 0) out vec4 fragColour;

layout(set = 0, binding = 0) uniform CameraUniform {
    Camera camera;
};
layout(set = 0, binding = 1) buffer ShapesData {
    float shapeValues[];
};

float rand(float at) {
    return fract(sin(at * 12.9898f * 43758.5453123f));
}

vec4 trace(Ray ray);
vec3 pointOnRay(in Ray ray, float t);

Sphere sphereAt(inout uint iShapeValue);

bool hitShape(in Ray ray, out vec4 color);
bool hitSphere(in Ray, in Sphere sphere, out Hit hit);

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

vec4 trace(Ray ray) {
    vec4 color;
    if (hitShape(ray, color)) {
        return color;
    }

    vec3 factor = 0.5f * (normalize(ray.direction) + 1.f);
    return vec4(mix(vec3(1.f), vec3(0.5f, 0.7f, 1.f), factor), 1.f);
}

vec3 pointOnRay(in Ray ray, float t) {
    return ray.position + (ray.direction * t);
}

Sphere sphereAt(inout uint iShapeValue) {
    return Sphere(
        vec4(shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++]),
        vec3(shapeValues[iShapeValue++], shapeValues[iShapeValue++], shapeValues[iShapeValue++]),
        shapeValues[iShapeValue++]
    );
}

bool hitShape(in Ray ray, out vec4 color) {
    bool hitOccured = false;
    uint iShapeValue = 0;

    float nShapeValues = shapeValues[iShapeValue++];
    float nSpheres = shapeValues[iShapeValue++];
    float lastDistance = FLOAT_MAX;
    for (float iSphere = 0.f; (iShapeValue < nShapeValues) && (iSphere < nSpheres); ++iSphere) {
        Sphere sphere = sphereAt(iShapeValue);

        Hit hit;
        if (hitSphere(ray, sphere, hit) && hit.distance < lastDistance) {
            hitOccured = true;
#if DRAW_NORMALS
            color = vec4(0.5f * (hit.normal + 1.f), 1.f);
#else
            // color = TODO
#endif
        }
    }
    return hitOccured;
}

bool hitSphere(in Ray ray, in Sphere sphere, out Hit hit) {
    vec3 oc = ray.position - sphere.position;
    float a = dot(ray.direction, ray.direction);
    float bHalved = dot(oc, ray.direction);
    float c = dot(oc, oc) - (sphere.radius * sphere.radius);
    float discriminant = (bHalved * bHalved) - (a * c);

    hit.distance = (-bHalved - sqrt(discriminant)) / a;
    hit.position = pointOnRay(ray, hit.distance);
    hit.normal = normalize(hit.position - sphere.position);

    return discriminant > 0.f;
}
