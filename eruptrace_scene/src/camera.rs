use crate::to_vec3;
use nalgebra_glm as glm;
use serde_json as js;

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

impl Camera {
    pub fn from_json(object: js::Value) -> anyhow::Result<Self> {
        let position = to_vec3(&object["position"]);
        let look_at = to_vec3(&object["look_at"]);
        let up = to_vec3(&object["up"]);
        let vertical_fov = object["fov"].as_f64().unwrap_or(90.0) as f32;
        let samples = object["samples"].as_u64().unwrap_or(1) as u32;
        let max_reflections = object["max_reflections"].as_u64().unwrap_or(1) as u32;

        Ok(Camera {
            position,
            look_at,
            up,
            img_size: [0, 0],
            vertical_fov,
            samples,
            max_reflections,
        })
    }
}
