use crate::materials::MaterialType;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Sphere {
    pub position: [f32; 3],
    pub radius: f32,
    pub material_type: MaterialType,
    pub material_index: u32,
}
