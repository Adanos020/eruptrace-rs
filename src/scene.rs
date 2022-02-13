use std::mem::size_of;
use crate::{materials::*, primitives::*};

pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub reflective_materials: Vec<ReflectiveMaterial>,
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
            data.push(sphere.radius);
            data.push(sphere.material_type as f32);
            data.push(sphere.material_index as f32);
        }

        data
    }

    pub fn get_material_data(&self) -> Vec<f32> {
        let mut data = Vec::with_capacity({
            let reflective_materials = self.reflective_materials.len() * size_of::<ReflectiveMaterial>();
            reflective_materials
        });

        for reflective in self.reflective_materials.iter() {
            data.push(reflective.color[0]);
            data.push(reflective.color[1]);
            data.push(reflective.color[2]);
            data.push(reflective.color[3]);
            data.push(reflective.fuzz);
        }

        data
    }
}
