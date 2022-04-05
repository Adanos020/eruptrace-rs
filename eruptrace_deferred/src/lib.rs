pub mod geometry_pass;
pub mod lighting_pass;

use crate::{geometry_pass::GeometryPass, lighting_pass::LightingPass};
use erupt::DeviceLoader;
use eruptrace_scene::{Camera, Scene};
use eruptrace_vk::{PipelineContext, VulkanContext};
use std::sync::{Arc, RwLock};
use vk_mem_erupt as vma;

pub struct DeferredRayTracer {
    geometry_pass: GeometryPass,
    lighting_pass: LightingPass,
}

impl DeferredRayTracer {
    pub fn new(
        allocator: Arc<RwLock<vma::Allocator>>,
        vk_ctx: VulkanContext,
        pipeline_ctx: PipelineContext,
        camera: Camera,
        scene: Scene,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            geometry_pass: GeometryPass::new(
                allocator,
                vk_ctx,
                pipeline_ctx,
                &camera,
                &scene.meshes,
            )?,
            lighting_pass: LightingPass::new(),
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.geometry_pass.destroy(device);
    }

    pub fn update_camera(&mut self, camera: Camera) {
        self.geometry_pass.update_camera(camera);
    }

    pub fn render(&self, vk_ctx: VulkanContext) {
        self.geometry_pass.render(vk_ctx);
    }
}
