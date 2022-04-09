#![allow(clippy::no_effect)]

pub mod geometry_pass;
pub mod lighting_pass;
pub mod shaders;

use erupt::DeviceLoader;
use eruptrace_scene::{Camera, CameraUniform, Mesh as SceneMesh, SceneBuffers};
use eruptrace_vk::{AllocatedBuffer, AllocatedImage, VulkanContext};

use crate::{geometry_pass::GeometryPass, lighting_pass::LightingPass};

pub struct DeferredRayTracer {
    geometry_pass: GeometryPass,
    lighting_pass: LightingPass,
}

impl DeferredRayTracer {
    pub fn new(
        vk_ctx: VulkanContext, camera: Camera, scene_meshes: Vec<SceneMesh>,
        camera_buffer: &AllocatedBuffer<CameraUniform>, scene_buffers: &SceneBuffers,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            geometry_pass: GeometryPass::new(vk_ctx.clone(), &camera, scene_meshes)?,
            lighting_pass: LightingPass::new(vk_ctx, camera_buffer, scene_buffers),
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.geometry_pass.destroy(device);
        self.lighting_pass.destroy(device);
    }

    pub fn update_camera(&mut self, vk_ctx: VulkanContext, camera: Camera) {
        self.geometry_pass.update_camera(vk_ctx, camera);
    }

    pub fn render(&self, vk_ctx: VulkanContext, target: &AllocatedImage) {
        self.geometry_pass.render(vk_ctx.clone());
        self.lighting_pass.render(vk_ctx, target);
    }
}
