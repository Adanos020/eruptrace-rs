#![allow(clippy::no_effect)]

pub mod geometry_pass;
pub mod lighting_pass;
pub mod shaders;

use erupt::{vk, DeviceLoader};
use eruptrace_scene::{Camera, Scene};
use eruptrace_vk::VulkanContext;

use crate::{geometry_pass::GeometryPass, lighting_pass::LightingPass};

pub struct DeferredRayTracer {
    geometry_pass: GeometryPass,
    lighting_pass: LightingPass,
}

impl DeferredRayTracer {
    pub fn new(vk_ctx: VulkanContext, camera: Camera, scene: Scene) -> anyhow::Result<Self> {
        Ok(Self {
            geometry_pass: GeometryPass::new(vk_ctx, &camera, &scene.meshes)?,
            lighting_pass: LightingPass::new(),
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.geometry_pass.destroy(device);
    }

    pub fn update_camera(&mut self, vk_ctx: VulkanContext, camera: Camera) {
        self.geometry_pass.update_camera(vk_ctx, camera);
    }

    pub fn render(&self, vk_ctx: VulkanContext, target: vk::ImageView) {
        self.geometry_pass.render(vk_ctx);
    }
}
