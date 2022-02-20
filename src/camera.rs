use nalgebra_glm as glm;
use std140::*;

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct CameraUniform {
    position: vec4,
    horizontal: vec4,
    vertical: vec4,
    bottom_left: vec4,
    img_size: vec2,
    img_size_inv: vec2,
    pub samples: uint,
    pub max_reflections: uint,
}

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
    pub fn to_uniform(&self) -> CameraUniform {
        let img_size = glm::vec2(self.img_size[0] as f32, self.img_size[1] as f32);
        let aspect = img_size.x / img_size.y;
        let half_height = (self.vertical_fov.to_radians() * 0.5).tan();
        let half_width = aspect * half_height;
        let focus_distance = glm::distance(&self.position, &self.look_at);

        let w = (self.position - self.look_at).normalize();
        let u = self.up.cross(&w).normalize();
        let v = w.cross(&u);

        let bottom_left = self.position
            - (half_width * focus_distance * u)
            - (half_height * focus_distance * v)
            - (focus_distance * w);
        let horizontal = 2.0 * half_width * focus_distance * u;
        let vertical = 2.0 * half_height * focus_distance * v;

        CameraUniform {
            position: vec4(self.position.x, self.position.y, self.position.z, 0.0),
            horizontal: vec4(horizontal.x, horizontal.y, horizontal.z, 0.0),
            vertical: vec4(vertical.x, vertical.y, vertical.z, 0.0),
            bottom_left: vec4(bottom_left.x, bottom_left.y, bottom_left.z, 0.0),
            img_size: vec2(img_size.x, img_size.y),
            img_size_inv: vec2(1.0 / img_size.x, 1.0 / img_size.y),
            samples: uint(self.samples),
            max_reflections: uint(self.max_reflections),
        }
    }
}
