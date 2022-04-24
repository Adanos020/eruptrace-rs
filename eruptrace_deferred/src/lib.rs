#![allow(clippy::no_effect)]

pub mod gbuffers;
pub mod geometry_pass;
pub mod lighting_pass;
pub mod shaders;

use erupt::DeviceLoader;
use eruptrace_scene::{Camera, CameraUniform, Mesh as SceneMesh, RtSceneBuffers};
use eruptrace_vk::{push_constants::RtPushConstants, AllocatedBuffer, AllocatedImage, VulkanContext};

use crate::{geometry_pass::GeometryPass, lighting_pass::LightingPass};

pub struct DeferredRayTracer {
    geometry_pass: GeometryPass,
    lighting_pass: LightingPass,
}

impl DeferredRayTracer {
    pub fn new(
        vk_ctx: VulkanContext,
        camera: Camera,
        scene_meshes: Vec<SceneMesh>,
        camera_buffer: &AllocatedBuffer<CameraUniform>,
        scene_buffers: &RtSceneBuffers,
    ) -> anyhow::Result<Self> {
        let output_extent = camera.image_extent_2d();
        let geometry_pass = GeometryPass::new(vk_ctx.clone(), &camera, scene_meshes)?;
        let lighting_pass =
            LightingPass::new(vk_ctx, output_extent, &geometry_pass.gbuffers, camera_buffer, scene_buffers);
        Ok(Self { geometry_pass, lighting_pass })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.geometry_pass.destroy(device);
        self.lighting_pass.destroy(device);
    }

    pub fn update_output(&mut self, vk_ctx: VulkanContext, camera: Camera) {
        self.geometry_pass.update_camera(vk_ctx.clone(), camera);
        self.lighting_pass.update_output(&vk_ctx.device, camera.image_extent_2d(), &self.geometry_pass.gbuffers);
    }

    pub fn render(&self, vk_ctx: VulkanContext, push_constants: &RtPushConstants, target: &AllocatedImage) {
        self.geometry_pass.render(vk_ctx.clone());
        self.lighting_pass.render(vk_ctx, push_constants, target);
    }
}
