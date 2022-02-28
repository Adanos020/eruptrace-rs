use crate::{materials::*, primitives::*, Scene};
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
        triangles: vec![],
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
        triangles: vec![],
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

pub fn cubes() -> Scene {
    Scene {
        spheres: vec![],
        triangles: vec![
            // Ground
            Triangle {
                vertices: [
                    PolygonVertex {
                        position: glm::vec3(-0.5, 0.0, -1.0),
                        normal: glm::vec3(0.0, 1.0, 0.0),
                        texture_coordinate: glm::vec2(0.0, 1.0),
                    },
                    PolygonVertex {
                        position: glm::vec3(0.5, 0.0, -1.0),
                        normal: glm::vec3(0.0, 1.0, 0.0),
                        texture_coordinate: glm::vec2(1.0, 1.0),
                    },
                    PolygonVertex {
                        position: glm::vec3(0.0, 0.0, -1.0),
                        normal: glm::vec3(0.0, 1.0, 0.0),
                        texture_coordinate: glm::vec2(0.5, 0.0),
                    },
                ],
                material_index: 0,
            }
        ],
        materials: vec![Material {
            material_type: MaterialType::Diffusive,
            texture_index: 1,
            parameter: 1.0,
        }],
        texture_paths: vec![
            "textures/sky.png".to_string(),
            "textures/sun.png".to_string(),
        ],
    }
}
