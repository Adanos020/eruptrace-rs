#![allow(clippy::no_effect)]

pub mod gbuffers;
pub mod geometry_pass;
pub mod lighting_pass;
pub mod shaders;

use erupt::{vk, DeviceLoader};
use eruptrace_scene::{Camera, CameraUniform, Mesh as SceneMesh, SceneBuffers};
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
        scene_buffers: &SceneBuffers,
    ) -> anyhow::Result<Self> {
        let output_extent = vk::Extent2D { width: camera.img_size[0], height: camera.img_size[1] };
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
        self.lighting_pass.update_output(
            &vk_ctx.device,
            vk::Extent2D { width: camera.img_size[0], height: camera.img_size[1] },
            &self.geometry_pass.gbuffers,
        );
    }

    pub fn render(&self, vk_ctx: VulkanContext, push_constants: &RtPushConstants, target: &AllocatedImage) {
        self.geometry_pass.render(vk_ctx.clone());
        self.lighting_pass.render(vk_ctx, push_constants, target);
    }
}
