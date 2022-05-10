#ifndef UTILS
#define UTILS

float rand(float at) {
    return fract(sin(dot((at + gl_FragCoord.xy), vec2(12.9898f, 78.233f))) * 43758.5453123f);
}

vec3 randPointInUnitCube(float at) {
    return vec3(
        -1.f + (2.f * rand(at)),
        -1.f + (2.f * rand(at + 1)),
        -1.f + (2.f * rand(at + 2)));
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
        1.f - ((asin(pointOnSphere.y) + HALF_PI) * ONE_OVER_PI));
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

#endif // UTILS
