use nalgebra_glm as glm;

pub fn vec2(v: &glm::Vec2) -> std140::vec2 {
    std140::vec2(v[0], v[1])
}

pub fn vec3(v: &glm::Vec3) -> std140::vec3 {
    std140::vec3(v[0], v[1], v[2])
}

pub fn vec4(v: &glm::Vec4) -> std140::vec4 {
    std140::vec4(v[0], v[1], v[2], v[3])
}

pub fn mat4x4(m: &glm::Mat4x4) -> std140::mat4x4 {
    std140::mat4x4(
        std140::vec4(m[(0, 0)], m[(1, 0)], m[(2, 0)], m[(3, 0)]),
        std140::vec4(m[(0, 1)], m[(1, 1)], m[(2, 1)], m[(3, 1)]),
        std140::vec4(m[(0, 2)], m[(1, 2)], m[(2, 2)], m[(3, 2)]),
        std140::vec4(m[(0, 3)], m[(1, 3)], m[(2, 3)], m[(3, 3)]),
    )
}
