#![allow(clippy::no_effect)]

use erupt::DeviceLoader;
use eruptrace_scene::{
    camera::{Camera, CameraUniform},
    Scene,
    SceneBuffers,
};
use eruptrace_vk::{
    contexts::{PipelineContext, RenderContext},
    AllocatedBuffer,
    VulkanContext,
};

use crate::render_surface::RenderSurface;

pub mod render_surface;
pub mod shaders;

#[derive(Clone)]
pub struct PureRayTracer {
    camera_buffer:  AllocatedBuffer<CameraUniform>,
    scene_buffers:  SceneBuffers,
    render_surface: RenderSurface,
}

impl PureRayTracer {
    pub fn new(vk_ctx: VulkanContext, pipeline_ctx: PipelineContext, camera: Camera, scene: Scene) -> Self {
        let camera_buffer = camera.into_uniform().create_buffer(vk_ctx.allocator.clone());
        let scene_buffers = scene.create_buffers(vk_ctx.clone());
        let render_surface = RenderSurface::new(vk_ctx, pipeline_ctx, &camera_buffer, &scene_buffers);

        Self { camera_buffer, scene_buffers, render_surface }
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.camera_buffer.destroy();
        self.scene_buffers.destroy(device);
        self.render_surface.destroy(device);
    }

    pub fn update_camera(&mut self, camera: Camera) {
        self.camera_buffer.set_data(&[camera.into_uniform()]);
    }

    pub fn render(&self, ctx: RenderContext) {
        self.render_surface.render(ctx);
    }
}
