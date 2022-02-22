use nalgebra_glm as glm;

#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub position: glm::TVec3<f32>,
    pub look_at: glm::TVec3<f32>,
    pub up: glm::TVec3<f32>,
    pub vertical_fov: f32,
    pub img_size: [u32; 2],
    pub samples: u32,
    pub max_reflections: u32,
}
