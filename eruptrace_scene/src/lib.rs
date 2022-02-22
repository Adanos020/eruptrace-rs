pub mod camera;
pub mod example_scenes;
pub mod materials;
pub mod primitives;

use crate::{materials::*, primitives::*};

pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub materials: Vec<Material>,
    pub texture_paths: Vec<String>,
}
