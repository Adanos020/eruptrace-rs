use std::mem::size_of;
use crate::{materials::*, primitives::*};
use std140::*;

pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub materials: Vec<Material>,
}

impl Scene {
    pub fn get_shape_data(&self) -> Vec<f32> {
        let mut data = Vec::with_capacity({
            let spheres = self.spheres.len() * size_of::<Sphere>();
            spheres
        });

        data.push(self.spheres.len() as f32);
        for sphere in self.spheres.iter() {
            data.push(sphere.position[0]);
            data.push(sphere.position[1]);
            data.push(sphere.position[2]);
            data.push(sphere.radius as f32);
            data.push(sphere.material_type as u32 as f32);
            data.push(sphere.material_index as f32);
        }

        data
    }

    pub fn test_scene() -> Self {
        Self {
            spheres: vec![
                Sphere { // Ground
                    position: [0.0, -100.5, -1.0],
                    radius: 100.0,
                    material_type: MaterialType::Diffusive,
                    material_index: 0,
                },
                Sphere { // Middle sphere
                    position: [0.0, 0.0, -1.0],
                    radius: 0.5,
                    material_type: MaterialType::Diffusive,
                    material_index: 1,
                },
                Sphere { // Middle back sphere
                    position: [0.0, 2.5, -6.5],
                    radius: 5.0,
                    material_type: MaterialType::Diffusive,
                    material_index: 2,
                },
                Sphere { // Left sphere
                    position: [-1.0, 0.0, -1.0],
                    radius: 0.5,
                    material_type: MaterialType::Reflective,
                    material_index: 3,
                },
                Sphere { // Right sphere
                    position: [1.0, 0.0, -1.0],
                    radius: 0.5,
                    material_type: MaterialType::Refractive,
                    material_index: 4,
                },
            ],
            materials: vec![
                Material { // Ground
                    color: vec4(0.2, 1.0, 0.6, 1.0),
                    parameter: float(1.0),
                },
                Material { // Middle sphere
                    color: vec4(1.0, 0.0, 0.0, 1.0),
                    parameter: float(1.0),
                },
                Material { // Middle back sphere
                    color: vec4(0.5, 0.5, 1.0, 1.0),
                    parameter: float(1.0),
                },
                Material { // Left sphere
                    color: vec4(0.8, 0.8, 0.8, 1.0),
                    parameter: float(0.1),
                },
                Material { // Right sphere
                    color: vec4(0.8, 0.8, 0.8, 1.0),
                    parameter: float(1.5),
                },
            ]
        }
    }
}
