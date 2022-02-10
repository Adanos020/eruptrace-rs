#version 450

layout(location = 0) out vec4 fragColour;

struct Sphere {
    vec3 position;
    float radius;
};

struct Ray {
    vec3 position;
    vec3 direction;
};

vec3 pointOnRay(in Ray ray, float t) {
    return ray.position + (ray.direction * t);
}

bool hitSphere(in Ray ray, in Sphere sphere, out float hitFactor) {
    vec3 oc = ray.position - sphere.position;
    float a = dot(ray.direction, ray.direction);
    float bHalved = dot(oc, ray.direction);
    float c = dot(oc, oc) - (sphere.radius * sphere.radius);
    float discriminant = (bHalved * bHalved) - (a * c);
    hitFactor = (-bHalved - sqrt(discriminant)) / a;
    return discriminant > 0.f;
}

vec4 trace(in Ray ray) {
    Sphere sphere;
    sphere.position = vec3(0.f, 0.f, -1.f);
    sphere.radius = 0.5f;

    float hitFactor;
    if (hitSphere(ray, sphere, hitFactor)) {
        vec3 normal = normalize(pointOnRay(ray, hitFactor) - sphere.position);
        normal.y *= -1.f;
        return vec4(0.5f * (normal + 1.f), 1.f);
    }

    vec3 factor = 0.5f * (normalize(ray.direction) + 1.f);
    return vec4(mix(vec3(1.f), vec3(0.5f, 0.7f, 1.f), factor), 1.f);
}

void main() {
    // TODO - Move these to push constants or uniforms.
    vec2 imgSize = vec2(1024, 1024);
    vec2 invImgSize = 1.f / imgSize;
    float aspectRatio = imgSize.x / imgSize.y;

    const float viewportHeight = 2.f;
    float viewportWidth = aspectRatio * viewportHeight;
    float focalLength = 1.f;

    vec3 camPos = vec3(0.f);
    vec3 horizontal = vec3(viewportWidth, 0.f, 0.f);
    vec3 vertical = vec3(0.f, viewportHeight, 0.f);
    vec3 lowerLeftCorner = camPos - (horizontal * 0.5f) - (vertical * 0.5f) - vec3(0.f, 0.f, focalLength);

    float u = gl_FragCoord.x * invImgSize.x;
    float v = (imgSize.y - gl_FragCoord.y) * invImgSize.y;

    // Ray tracer
    Ray ray;
    ray.position = vec3(0.f);
    ray.direction = lowerLeftCorner + (u * horizontal) + (v * vertical) - camPos;

    vec4 out_colour = trace(ray);
    fragColour = out_colour;
}
