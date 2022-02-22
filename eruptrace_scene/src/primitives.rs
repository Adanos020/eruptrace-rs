use crate::materials::MaterialType;
use nalgebra_glm as glm;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Sphere {
    pub position: glm::TVec3<f32>,
    pub radius: f32,
    pub material_type: MaterialType,
    pub material_index: u32,
}
