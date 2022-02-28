pub mod camera;
pub mod example_scenes;
pub mod materials;
pub mod shapes;

use crate::{materials::*, shapes::*};

pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub texture_paths: Vec<String>,
}
