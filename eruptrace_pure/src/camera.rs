#![allow(clippy::no_effect)]

use eruptrace_scene::camera::Camera;
use nalgebra_glm as glm;
use std::sync::Arc;
use std140::*;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    device::Device,
};

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

impl CameraUniform {
    pub fn to_buffer(self, device: Arc<Device>) -> Arc<CpuAccessibleBuffer<CameraUniform>> {
        CpuAccessibleBuffer::from_data(device, BufferUsage::uniform_buffer(), false, self)
            .expect("Cannot create uniform buffer for camera.")
    }
}

impl From<Camera> for CameraUniform {
    fn from(camera: Camera) -> Self {
        let img_size = glm::vec2(camera.img_size[0] as f32, camera.img_size[1] as f32);
        let aspect = img_size.x / img_size.y;
        let half_height = (camera.vertical_fov.to_radians() * 0.5).tan();
        let half_width = aspect * half_height;
        let focus_distance = glm::distance(&camera.position, &camera.look_at);

        let w = (camera.position - camera.look_at).normalize();
        let u = camera.up.cross(&w).normalize();
        let v = w.cross(&u);

        let bottom_left = camera.position
            - (half_width * focus_distance * u)
            - (half_height * focus_distance * v)
            - (focus_distance * w);
        let horizontal = 2.0 * half_width * focus_distance * u;
        let vertical = 2.0 * half_height * focus_distance * v;

        Self {
            position: vec4(camera.position.x, camera.position.y, camera.position.z, 0.0),
            horizontal: vec4(horizontal.x, horizontal.y, horizontal.z, 0.0),
            vertical: vec4(vertical.x, vertical.y, vertical.z, 0.0),
            bottom_left: vec4(bottom_left.x, bottom_left.y, bottom_left.z, 0.0),
            img_size: vec2(img_size.x, img_size.y),
            img_size_inv: vec2(1.0 / img_size.x, 1.0 / img_size.y),
            samples: uint(camera.samples),
            max_reflections: uint(camera.max_reflections),
        }
    }
}
