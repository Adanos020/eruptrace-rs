#version 450

layout(location = 0) out vec4 fragColour;

struct Camera {
    vec4 position;
    vec4 horizontal;
    vec4 vertical;
    vec4 bottomLeft;
    vec2 imgSize;
    vec2 imgSizeInv;
};

layout(set = 0, binding = 0) uniform CameraUniform {
    Camera camera;
};

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
    float u = gl_FragCoord.x * camera.imgSizeInv.x;
    float v = (camera.imgSize.y - gl_FragCoord.y) * camera.imgSizeInv.y;
    vec3 samplePosition = (camera.bottomLeft + (u * camera.horizontal) + (v * camera.vertical)).xyz;

    // Ray tracer
    Ray ray;
    ray.position = vec3(0.f);
    ray.direction = samplePosition - camera.position.xyz;

    vec4 out_colour = trace(ray);
    fragColour = out_colour;
}
