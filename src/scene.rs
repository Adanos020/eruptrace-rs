use crate::{materials::*, primitives::*};
use image::EncodableLayout;
use nalgebra_glm as glm;
use std::{mem::size_of, sync::Arc};
use std140::*;
use vulkano::{
    device::{Device, Queue},
    format::Format,
    image::{ImageDimensions, ImmutableImage, MipmapsCount},
    sync::GpuFuture,
};

pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub materials: Vec<Material>,
    pub texture_paths: Vec<String>,
}

impl Scene {
    pub fn test_scene() -> Self {
        Self {
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
                    texture_index: uint(4),
                    parameter: float(1.0),
                },
                Material {
                    // Middle sphere
                    texture_index: uint(1),
                    parameter: float(1.0),
                },
                Material {
                    // Middle back sphere
                    texture_index: uint(2),
                    parameter: float(1.0),
                },
                Material {
                    // Left sphere
                    texture_index: uint(3),
                    parameter: float(0.1),
                },
                Material {
                    // Right sphere
                    texture_index: uint(3),
                    parameter: float(1.5),
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

    pub fn test_dark_scene() -> Self {
        Self {
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
                    texture_index: uint(4),
                    parameter: float(1.0),
                },
                Material {
                    // Light
                    texture_index: uint(1),
                    parameter: float(5.0),
                },
                Material {
                    // Middle sphere
                    texture_index: uint(2),
                    parameter: float(1.0),
                },
                Material {
                    // Left sphere
                    texture_index: uint(3),
                    parameter: float(0.1),
                },
                Material {
                    // Right sphere
                    texture_index: uint(3),
                    parameter: float(1.5),
                },
            ],
            texture_paths: vec![
                "textures/sky_night.png".to_string(),
                "textures/sun.png".to_string(),
                "textures/earth.png".to_string(),
                "textures/gray.png".to_string(),
            ],
        }
    }

    pub fn get_shape_data(&self) -> Vec<f32> {
        let mut data = Vec::with_capacity(self.spheres.len() * size_of::<Sphere>());

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

    pub fn get_texture_data(&self, device: Arc<Device>, queue: Arc<Queue>) -> Arc<ImmutableImage> {
        let texture_bytes: Vec<u8> = self
            .texture_paths
            .iter()
            .map(|path| image::open(path).unwrap().into_rgba8())
            .flat_map(|texture| Vec::from(texture.as_bytes()))
            .collect();

        let (textures, future) = ImmutableImage::from_iter(
            texture_bytes,
            ImageDimensions::Dim2d {
                width: 1024,
                height: 1024,
                array_layers: self.texture_paths.len() as u32,
            },
            MipmapsCount::One,
            Format::R8G8B8A8_UNORM,
            queue,
        )
        .expect("Cannot create textures image.");

        vulkano::sync::now(device)
            .join(future)
            .then_signal_fence_and_flush()
            .expect("Cannot flush.")
            .wait(None)
            .expect("Cannot wait.");

        textures
    }
}
