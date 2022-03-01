use crate::{materials::*, shapes::*, Scene};
use nalgebra_glm as glm;

pub fn spheres_day() -> Scene {
    Scene {
        spheres: vec![
            Sphere {
                // Ground
                position: glm::vec3(0.0, -200.5, -1.0),
                radius: 200.0,
                material_index: 0,
            },
            Sphere {
                // Middle back sphere
                position: glm::vec3(0.0, 4.5, -6.5),
                radius: 5.0,
                material_index: 2,
            },
            Sphere {
                // Middle sphere
                position: glm::vec3(0.0, 0.0, -1.0),
                radius: 0.5,
                material_index: 1,
            },
            Sphere {
                // Left sphere
                position: glm::vec3(-1.0, 0.0, -1.0),
                radius: 0.5,
                material_index: 3,
            },
            Sphere {
                // Right sphere
                position: glm::vec3(1.0, 0.0, -1.0),
                radius: 0.5,
                material_index: 4,
            },
        ],
        meshes: vec![],
        materials: vec![
            Material {
                // Ground
                material_type: MaterialType::Diffusive,
                texture_index: 4,
                parameter: 1.0,
            },
            Material {
                // Middle sphere
                material_type: MaterialType::Diffusive,
                texture_index: 1,
                parameter: 1.0,
            },
            Material {
                // Middle back sphere
                material_type: MaterialType::Diffusive,
                texture_index: 2,
                parameter: 1.0,
            },
            Material {
                // Left sphere
                material_type: MaterialType::Reflective,
                texture_index: 3,
                parameter: 0.1,
            },
            Material {
                // Right sphere
                material_type: MaterialType::Refractive,
                texture_index: 3,
                parameter: 1.5,
            },
        ],
        texture_paths: vec![
            "textures/sky.png".to_string(),
            "textures/earth.png".to_string(),
            "textures/jupiter.png".to_string(),
            "textures/gray.png".to_string(),
            "textures/venus.png".to_string(),
        ],
    }
}

pub fn spheres_night() -> Scene {
    Scene {
        spheres: vec![
            Sphere {
                // Ground
                position: glm::vec3(0.0, -200.5, -1.0),
                radius: 200.0,
                material_index: 0,
            },
            Sphere {
                // Light
                position: glm::vec3(0.0, 1.5, -6.5),
                radius: 2.0,
                material_index: 1,
            },
            Sphere {
                // Middle sphere
                position: glm::vec3(0.0, 0.0, -1.0),
                radius: 0.5,
                material_index: 2,
            },
            Sphere {
                // Left sphere
                position: glm::vec3(-1.0, 0.0, -1.0),
                radius: 0.5,
                material_index: 3,
            },
            Sphere {
                // Right sphere
                position: glm::vec3(1.0, 0.0, -1.0),
                radius: 0.5,
                material_index: 4,
            },
        ],
        meshes: vec![],
        materials: vec![
            Material {
                // Ground
                material_type: MaterialType::Diffusive,
                texture_index: 4,
                parameter: 1.0,
            },
            Material {
                // Light
                material_type: MaterialType::Emitting,
                texture_index: 1,
                parameter: 5.0,
            },
            Material {
                // Middle sphere
                material_type: MaterialType::Diffusive,
                texture_index: 2,
                parameter: 1.0,
            },
            Material {
                // Left sphere
                material_type: MaterialType::Reflective,
                texture_index: 3,
                parameter: 0.1,
            },
            Material {
                // Right sphere
                material_type: MaterialType::Refractive,
                texture_index: 3,
                parameter: 1.5,
            },
        ],
        texture_paths: vec![
            "textures/sky_night.png".to_string(),
            "textures/sun.png".to_string(),
            "textures/earth.png".to_string(),
            "textures/gray.png".to_string(),
            "textures/jupiter.png".to_string(),
        ],
    }
}

pub fn cube() -> Scene {
    Scene {
        spheres: vec![],
        meshes: vec![
            // Ground
            Mesh {
                positions: vec![
                    glm::vec3(-10.0, 0.0, 10.0),
                    glm::vec3(10.0, 0.0, 10.0),
                    glm::vec3(10.0, 0.0, -10.0),
                    glm::vec3(-10.0, 0.0, -10.0),
                ],
                normals: [glm::vec3(0.0, 1.0, 0.0); 4].to_vec(),
                texcoords: vec![
                    glm::vec2(0.0, 10.0),
                    glm::vec2(10.0, 10.0),
                    glm::vec2(10.0, 0.0),
                    glm::vec2(0.0, 0.0),
                ],
                indices: vec![0, 1, 2, 0, 2, 3],
                material_index: 0,
            },
            // Cube
            Mesh {
                positions: vec![
                    // Left
                    glm::vec3(0.0, 0.0, 0.0),
                    glm::vec3(0.0, 1.0, 0.0),
                    glm::vec3(0.0, 1.0, 1.0),
                    glm::vec3(0.0, 0.0, 1.0),
                    // Right
                    glm::vec3(1.0, 0.0, 0.0),
                    glm::vec3(1.0, 1.0, 0.0),
                    glm::vec3(1.0, 1.0, 1.0),
                    glm::vec3(1.0, 0.0, 1.0),
                    // Bottom
                    glm::vec3(0.0, 0.0, 1.0),
                    glm::vec3(1.0, 0.0, 1.0),
                    glm::vec3(1.0, 0.0, 0.0),
                    glm::vec3(0.0, 0.0, 0.0),
                    // Top
                    glm::vec3(0.0, 1.0, 1.0),
                    glm::vec3(1.0, 1.0, 1.0),
                    glm::vec3(1.0, 1.0, 0.0),
                    glm::vec3(0.0, 1.0, 0.0),
                    // Back
                    glm::vec3(0.0, 0.0, 0.0),
                    glm::vec3(1.0, 0.0, 0.0),
                    glm::vec3(1.0, 1.0, 0.0),
                    glm::vec3(0.0, 1.0, 0.0),
                    // Front
                    glm::vec3(0.0, 0.0, 1.0),
                    glm::vec3(1.0, 0.0, 1.0),
                    glm::vec3(1.0, 1.0, 1.0),
                    glm::vec3(0.0, 1.0, 1.0),
                ],
                normals: [
                    [glm::vec3(-1.0, 0.0, 0.0); 4],
                    [glm::vec3(1.0, 0.0, 0.0); 4],
                    [glm::vec3(0.0, -1.0, 0.0); 4],
                    [glm::vec3(0.0, 1.0, 0.0); 4],
                    [glm::vec3(0.0, 0.0, -1.0); 4],
                    [glm::vec3(0.0, 0.0, 1.0); 4],
                ]
                .into_iter()
                .flatten()
                .collect(),
                texcoords: [[
                    glm::vec2(0.0, 1.0),
                    glm::vec2(1.0, 1.0),
                    glm::vec2(1.0, 0.0),
                    glm::vec2(0.0, 0.0),
                ]; 6]
                    .into_iter()
                    .flatten()
                    .collect(),
                indices: vec![
                    0, 1, 3, 1, 2, 3, // Left
                    4, 5, 7, 5, 6, 7, // Right
                    8, 9, 11, 9, 10, 11, // Bottom
                    12, 13, 15, 13, 14, 15, // Top
                    16, 17, 19, 17, 18, 19, // Back
                    20, 21, 23, 21, 22, 23, // Front
                ],
                material_index: 1,
            },
        ],
        materials: vec![
            // Ground
            Material {
                material_type: MaterialType::Diffusive,
                texture_index: 1,
                parameter: 1.0,
            },
            // Cube
            Material {
                material_type: MaterialType::Diffusive,
                texture_index: 2,
                parameter: 1.0,
            },
        ],
        texture_paths: vec![
            "textures/sky.png".to_string(),
            "textures/grass.png".to_string(),
            "textures/bricks.png".to_string(),
        ],
    }
}
