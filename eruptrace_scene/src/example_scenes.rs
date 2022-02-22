use crate::{materials::*, primitives::*, Scene};
use nalgebra_glm as glm;

pub fn spheres_day() -> Scene {
    Scene {
        spheres: vec![
            Sphere {
                // Ground
                position: glm::vec3(0.0, -200.5, -1.0),
                radius: 200.0,
                material_type: MaterialType::Diffusive,
                material_index: 0,
            },
            Sphere {
                // Middle back sphere
                position: glm::vec3(0.0, 4.5, -6.5),
                radius: 5.0,
                material_type: MaterialType::Diffusive,
                material_index: 2,
            },
            Sphere {
                // Middle sphere
                position: glm::vec3(0.0, 0.0, -1.0),
                radius: 0.5,
                material_type: MaterialType::Diffusive,
                material_index: 1,
            },
            Sphere {
                // Left sphere
                position: glm::vec3(-1.0, 0.0, -1.0),
                radius: 0.5,
                material_type: MaterialType::Reflective,
                material_index: 3,
            },
            Sphere {
                // Right sphere
                position: glm::vec3(1.0, 0.0, -1.0),
                radius: 0.5,
                material_type: MaterialType::Refractive,
                material_index: 4,
            },
        ],
        materials: vec![
            Material {
                // Ground
                texture_index: 4,
                parameter: 1.0,
            },
            Material {
                // Middle sphere
                texture_index: 1,
                parameter: 1.0,
            },
            Material {
                // Middle back sphere
                texture_index: 2,
                parameter: 1.0,
            },
            Material {
                // Left sphere
                texture_index: 3,
                parameter: 0.1,
            },
            Material {
                // Right sphere
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
                material_type: MaterialType::Diffusive,
                material_index: 0,
            },
            Sphere {
                // Light
                position: glm::vec3(0.0, 1.5, -6.5),
                radius: 2.0,
                material_type: MaterialType::Emitting,
                material_index: 1,
            },
            Sphere {
                // Middle sphere
                position: glm::vec3(0.0, 0.0, -1.0),
                radius: 0.5,
                material_type: MaterialType::Diffusive,
                material_index: 2,
            },
            Sphere {
                // Left sphere
                position: glm::vec3(-1.0, 0.0, -1.0),
                radius: 0.5,
                material_type: MaterialType::Reflective,
                material_index: 3,
            },
            Sphere {
                // Right sphere
                position: glm::vec3(1.0, 0.0, -1.0),
                radius: 0.5,
                material_type: MaterialType::Refractive,
                material_index: 4,
            },
        ],
        materials: vec![
            Material {
                // Ground
                texture_index: 4,
                parameter: 1.0,
            },
            Material {
                // Light
                texture_index: 1,
                parameter: 5.0,
            },
            Material {
                // Middle sphere
                texture_index: 2,
                parameter: 1.0,
            },
            Material {
                // Left sphere
                texture_index: 3,
                parameter: 0.1,
            },
            Material {
                // Right sphere
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
