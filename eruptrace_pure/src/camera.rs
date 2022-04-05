#![allow(clippy::no_effect)]

use erupt::vk;
use eruptrace_scene::camera::Camera;
use eruptrace_vk::AllocatedBuffer;
use nalgebra_glm as glm;
use std::sync::{Arc, RwLock};
use std140::*;
use vk_mem_erupt as vma;

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
    pub fn create_buffer(
        self,
        allocator: Arc<RwLock<vma::Allocator>>,
    ) -> AllocatedBuffer<Self> {
        let buffer_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        AllocatedBuffer::with_data(allocator, &buffer_info, vma::MemoryUsage::CpuToGpu, &[self])
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
