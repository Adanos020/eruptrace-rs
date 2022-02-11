use crate::primitives::Sphere;

pub struct Scene {
    pub spheres: Vec<Sphere>,
}

impl Scene {
    pub fn get_shape_data(&self) -> Vec<f32> {
        let mut data = Vec::with_capacity(1 + self.spheres.len() * std::mem::size_of::<Sphere>());
        data.push(data.capacity() as f32);
        data.push(self.spheres.len() as f32);
        for sphere in self.spheres.iter() {
            data.push(sphere.color[0]);
            data.push(sphere.color[1]);
            data.push(sphere.color[2]);
            data.push(sphere.color[3]);
            data.push(sphere.position[0]);
            data.push(sphere.position[1]);
            data.push(sphere.position[2]);
            data.push(sphere.radius);
        }
        data
    }
}
