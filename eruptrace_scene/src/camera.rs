use std::sync::{Arc, RwLock};

use erupt::vk;
use eruptrace_vk::AllocatedBuffer;
use nalgebra_glm as glm;
use serde_json as js;
use std140::repr_std140;
use vk_mem_erupt as vma;

use crate::to_vec3;

#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub position:        glm::Vec3,
    pub look_at:         glm::Vec3,
    pub up:              glm::Vec3,
    pub vertical_fov:    f32,
    pub img_size:        [u32; 2],
    pub samples:         u32,
    pub max_reflections: u32,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct CameraUniform {
    position:            std140::vec4,
    horizontal:          std140::vec4,
    vertical:            std140::vec4,
    bottom_left:         std140::vec4,
    img_size:            std140::vec2,
    img_size_inv:        std140::vec2,
    pub samples:         std140::uint,
    pub max_reflections: std140::uint,
}

impl Camera {
    pub fn from_json(object: js::Value) -> anyhow::Result<Self> {
        let position = to_vec3(&object["position"]).unwrap();
        let look_at = to_vec3(&object["look_at"]).unwrap();
        let up = to_vec3(&object["up"]).unwrap();
        let vertical_fov = object["fov"].as_f64().unwrap_or(90.0) as f32;
        let samples = object["samples"].as_u64().unwrap_or(1) as u32;
        let max_reflections = object["max_reflections"].as_u64().unwrap_or(1) as u32;

        Ok(Camera { position, look_at, up, img_size: [0, 0], vertical_fov, samples, max_reflections })
    }

    pub fn into_uniform(self) -> CameraUniform {
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
            position:        std140::vec4(self.position.x, self.position.y, self.position.z, 0.0),
            horizontal:      std140::vec4(horizontal.x, horizontal.y, horizontal.z, 0.0),
            vertical:        std140::vec4(vertical.x, vertical.y, vertical.z, 0.0),
            bottom_left:     std140::vec4(bottom_left.x, bottom_left.y, bottom_left.z, 0.0),
            img_size:        std140::vec2(img_size.x, img_size.y),
            img_size_inv:    std140::vec2(1.0 / img_size.x, 1.0 / img_size.y),
            samples:         std140::uint(self.samples),
            max_reflections: std140::uint(self.max_reflections),
        }
    }
}

impl CameraUniform {
    pub fn create_buffer(self, allocator: Arc<RwLock<vma::Allocator>>) -> AllocatedBuffer<Self> {
        let buffer_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        AllocatedBuffer::with_data(allocator, &buffer_info, vma::MemoryUsage::CpuToGpu, &[self])
    }
}
