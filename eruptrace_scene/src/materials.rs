#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum MaterialType {
    Diffusive = 0,
    Reflective = 1,
    Refractive = 2,
    Emitting = 3,
}

#[derive(Copy, Clone, Debug)]
pub struct Material {
    pub texture_index: u32,

    /// The role of this parameter depends on the material type:
    /// - Diffusive: no function
    /// - Reflective: fuzz
    /// - Refractive: refractive index
    /// - Emitting: intensity
    pub parameter: f32,
}
