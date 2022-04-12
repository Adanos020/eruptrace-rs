#version 450

#include <structs.glsl>

layout(location = 0) out vec4 fragColour;

layout(set = 0, binding = 0) uniform sampler2D inPositions;
layout(set = 0, binding = 1) uniform sampler2D inNormals;
layout(set = 0, binding = 2) uniform sampler2D inMaterials;

layout(set = 1, binding = 0) uniform sampler2DArray textures;
layout(set = 1, binding = 1) uniform sampler2DArray normalMaps;
layout(set = 1, binding = 2) uniform CameraUniform {
    vec4 position;
    vec4 horizontal;
    vec4 vertical;
    vec4 bottomLeft;
    vec2 imgSize;
    vec2 imgSizeInv;
    uint sqrtSamples;
    uint maxReflections;
} camera;
layout(set = 1, binding = 3, std140) readonly buffer BIH {
    BihNode bihNodes[];
};
layout(set = 1, binding = 4, std140) readonly buffer MaterialData {
    Material materials[];
};
layout(set = 1, binding = 5, std140) readonly buffer TriangleData {
    Triangle triangles[];
};

layout(push_constant) uniform Constants {
    uint nTriangles;
    bool bUseBih;
    bool bRenderNormals;
};

#include <ray_tracing.glsl>

void main() {
    vec2 imgSize = camera.imgSize * camera.sqrtSamples;
    vec2 imgSizeInv = 1.f / imgSize;
    float sqrtSamplesInv = 1.f / camera.sqrtSamples;
    vec4 finalColor = vec4(0.f);
    for (uint y = 0; y < camera.sqrtSamples; ++y) {
        for (uint x = 0; x < camera.sqrtSamples; ++x) {
            float u = (gl_FragCoord.x + (x * sqrtSamplesInv)) * camera.imgSizeInv.x;
            float v = (camera.imgSize.y - gl_FragCoord.y - (y * sqrtSamplesInv)) * camera.imgSizeInv.y;
            vec4 samplePosition = camera.bottomLeft + (u * camera.horizontal) + (v * camera.vertical);

            Ray ray;
            ray.origin = camera.position.xyz;
            ray.direction = (samplePosition - camera.position).xyz;
            ray.invDirection = 1.f / ray.direction;

            vec2 gUV = ((gl_FragCoord.xy * camera.sqrtSamples) + vec2(x, y)) * imgSizeInv;
            vec4 position = texture(inPositions, gUV);
            bool hitOccured = position.w == 1.f;
            if (hitOccured) {
                // Geometry
                vec4 material = texture(inMaterials, gUV);
                vec3 normal = texture(inNormals, gUV).xyz;
                float dotRayNorm = dot(ray.direction, normal);

                Hit hit;
                hit.position = position.xyz;
                hit.normal = normal;
                hit.texCoords = material.xy;
                hit.incidental = ray.direction;
                hit.materialIndex = uint(material.z);
                hit.bFrontFace = dotRayNorm < 0.f;

                Scattering initialScattering;
                if (scatter(hit, initialScattering)) {
                    finalColor += initialScattering.color * trace(initialScattering.newRay);
                } else {
                    finalColor += initialScattering.color;
                }
            } else {
                // Sky
                vec3 rayDir = normalize(ray.direction);
                finalColor += sampleTexture(mappingOnUnitSphere(rayDir), 0);
            }
        }
    }
    fragColour = sqrt(finalColor / float(camera.sqrtSamples * camera.sqrtSamples));
}
