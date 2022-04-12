#version 450

#include <structs.glsl>

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
    uint flags;
};

#include <ray_tracing.glsl>

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
